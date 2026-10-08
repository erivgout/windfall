import type { ChannelId, PatternId } from "@/bindings"
import {
  clampViewport,
  zoomRowsAt,
  type TimeGridView,
  type Viewport,
} from "@/lib/canvas"

import { onProjectReplaced } from "@/lib/store/replaced"
import { TICKS_PER_STEP } from "@/lib/units"

import { notesExtent, type Extent, type PasteTarget } from "./edit-math"
import type { Editor } from "./editor"
import {
  fitViewport,
  openingViewport,
  ROW_COUNT,
  steppedRowHeight,
} from "./view-math"

type SavedView = Pick<
  Viewport,
  "scrollTick" | "scrollRow" | "pxPerTick" | "rowHeight"
>

// The panel is unmounted whenever another editor tab shows, so where each
// lane was scrolled to is kept here.
const savedViews = new Map<string, SavedView>()
let lastZoom: Pick<Viewport, "pxPerTick" | "rowHeight"> | null = null
// Goes up when the kept views are forgotten. A session from before that
// has nothing to add to them any more.
let savedEpoch = 0

export function forgetSavedViews() {
  savedViews.clear()
  lastZoom = null
  savedEpoch += 1
}

// Lanes are known by pattern and channel id, and both start over in every
// project: the view kept for "1:2" belongs to the song before.
onProjectReplaced(forgetSavedViews)

/**
 * One open piano roll: the editor, the canvas view once it exists, and the
 * few values the pieces around the grid share while they change every
 * frame. Nothing here goes through React state.
 */
export class PianoRollSession {
  readonly editor: Editor
  view: TimeGridView | null = null
  /** The pattern length shown while the end marker is being dragged. */
  lengthPreview: number | null = null
  /** Which lane is open. The editor's host reads the project with it. */
  editing: { patternId: PatternId; channelId: ChannelId } | null = null
  /** The playhead inside the pattern, or null while it is not shown. */
  playhead: number | null = null
  private listeners = new Set<() => void>()
  private playheadListeners = new Set<() => void>()
  private stopView: (() => void) | null = null
  private focusTarget: (() => HTMLElement | null) | null = null
  private laneKey: string | null = null
  private readonly epoch = savedEpoch
  private stopProject: (() => void) | null = null

  constructor(editor: Editor) {
    this.editor = editor
  }

  /** Called when the viewport, the theme, the view or a preview changes. */
  onView(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  notifyView(): void {
    for (const listener of [...this.listeners]) listener()
  }

  setEditing(patternId: PatternId, channelId: ChannelId): void {
    this.stopProject ??= onProjectReplaced(() => this.editor.cancel())
    this.editing = { patternId, channelId }
  }

  /** Says which element takes the keyboard focus for the grid. */
  setFocusTarget(target: (() => HTMLElement | null) | null): void {
    this.focusTarget = target
  }

  /** Puts the keyboard focus on the grid, so shortcuts go to the notes. */
  focusGrid(): void {
    this.focusTarget?.()?.focus({ preventScroll: true })
  }

  /** Shows a pattern length that is not committed yet, or null to stop. */
  setLengthPreview(ticks: number | null): void {
    if (ticks === this.lengthPreview) return
    this.lengthPreview = ticks
    this.notifyView()
    this.view?.invalidate("overlay")
  }

  /** Called when the playhead moves, which is every frame while playing. */
  onPlayhead(listener: () => void): () => void {
    this.playheadListeners.add(listener)
    return () => {
      this.playheadListeners.delete(listener)
    }
  }

  setPlayhead(tick: number | null): void {
    if (tick === this.playhead) return
    this.playhead = tick
    this.view?.setPlayhead(tick)
    for (const listener of [...this.playheadListeners]) listener()
  }

  attachView(view: TimeGridView | null): void {
    this.stopView?.()
    this.stopView = null
    this.saveView()
    this.view = view
    this.editor.attach(view)
    if (view) {
      view.setPlayhead(this.playhead)
      const offViewport = view.onViewportChange(() => this.notifyView())
      const offTheme = view.onThemeChange(() => this.notifyView())
      this.stopView = () => {
        offViewport()
        offTheme()
      }
    }
    this.notifyView()
  }

  /**
   * Points the view at a lane. A lane seen before comes back where it was
   * left; a new one opens on its notes, or around C5 when it is empty.
   */
  showLane(key: string): void {
    const view = this.view
    if (!view) return
    if (this.laneKey !== null && this.laneKey !== key) this.saveView()
    this.laneKey = key
    const saved = savedViews.get(key)
    const zoomed = { ...view.viewport, ...lastZoom }
    view.setViewport(
      saved
        ? { ...view.viewport, ...saved }
        : openingViewport(zoomed, this.editor.notes, view.limits)
    )
  }

  saveView(): void {
    const view = this.view
    if (!view || this.laneKey === null || this.epoch !== savedEpoch) return
    const { scrollTick, scrollRow, pxPerTick, rowHeight } = view.viewport
    savedViews.set(this.laneKey, {
      scrollTick,
      scrollRow,
      pxPerTick,
      rowHeight,
    })
    lastZoom = { pxPerTick, rowHeight }
  }

  /** Editing a ghost keeps the shared grid rather than jumping to a saved lane. */
  keepViewportForLane(key: string): void {
    this.saveView()
    this.laneKey = key
    this.saveView()
  }

  zoomToFit(): void {
    const extent = notesExtent(this.editor.notes) ?? this.emptyExtent()
    this.fit(extent)
  }

  zoomToSelection(): void {
    const extent = notesExtent(this.editor.selectedNotes())
    if (extent) this.fit(extent)
  }

  /** Zooms row height one notch around a y coordinate inside the grid. */
  zoomRows(anchorY: number, factor: number): void {
    const view = this.view
    if (!view) return
    const { rowHeight } = view.viewport
    const next = steppedRowHeight(rowHeight, factor)
    if (next === rowHeight) return
    view.setViewport(
      zoomRowsAt(view.viewport, anchorY, next / rowHeight, view.limits)
    )
  }

  /** Sets the visible part of the content from fractions, as a scrollbar does. */
  setRange(axis: "time" | "rows", start: number, end: number): void {
    const view = this.view
    if (!view) return
    const viewport = view.viewport
    const share = Math.max(0.0001, end - start)
    if (axis === "time") {
      const total = view.limits.contentTicks
      const pxPerTick = Math.min(
        view.limits.maxPxPerTick,
        Math.max(view.limits.minPxPerTick, viewport.width / (share * total))
      )
      view.setViewport({ ...viewport, pxPerTick, scrollTick: start * total })
      return
    }
    const rowHeight = Math.min(
      view.limits.maxRowHeight,
      Math.max(
        view.limits.minRowHeight,
        Math.round(viewport.height / (share * ROW_COUNT))
      )
    )
    view.setViewport(
      clampViewport(
        { ...viewport, rowHeight, scrollRow: start * ROW_COUNT },
        view.limits
      )
    )
  }

  /** Scrolls so the visible part starts at a fraction of the content. */
  scrollTo(axis: "time" | "rows", start: number): void {
    const view = this.view
    if (!view) return
    view.setViewport(
      axis === "time"
        ? {
            ...view.viewport,
            scrollTick: start * view.limits.contentTicks,
          }
        : { ...view.viewport, scrollRow: start * ROW_COUNT }
    )
  }

  /** Where a paste goes: the playhead when it shows, or the first bar in view. */
  pasteTarget(): PasteTarget {
    if (this.playhead !== null) return { at: "playhead", tick: this.playhead }
    return { at: "view", leftTick: this.view?.viewport.scrollTick ?? 0 }
  }

  dispose(): void {
    this.stopProject?.()
    this.stopProject = null
    this.saveView()
    this.stopView?.()
    this.stopView = null
    this.view = null
    this.editor.attach(null)
  }

  private fit(extent: Extent): void {
    const view = this.view
    if (!view) return
    view.setViewport(fitViewport(view.viewport, extent, view.limits))
  }

  private emptyExtent(): Extent {
    const ctx = this.editor.context
    const end = (ctx?.pattern.lengthSteps ?? 16) * TICKS_PER_STEP
    return { start: 0, end, lowKey: 54, highKey: 66 }
  }
}

let current: PianoRollSession | null = null

/** The piano roll that is open, for actions. Null while the panel is not mounted. */
export function currentSession(): PianoRollSession | null {
  return current
}

export function setCurrentSession(session: PianoRollSession | null) {
  current = session
}
