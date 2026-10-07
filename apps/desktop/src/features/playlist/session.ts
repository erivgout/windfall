import type { Clip, ClipId, Pattern } from "@/bindings"
import { queryRect, xToTick, yToRow } from "@/lib/canvas"
import { clamp } from "@/lib/units"

import {
  clampMove,
  cloneMoved,
  endResizeDelta,
  minResizeLength,
  moveChanges,
  paintStarts,
  patternTicks,
  resizeChanges,
  snapCell,
  startTrimDelta,
  strokeBox,
  withoutStacked,
  type NewClip,
} from "./edit"
import { cursorFor, intentFor } from "./intents"
import { DOUBLE_CLICK_MS, DRAG_THRESHOLD_PX, edgePull } from "./layout"
import type { GridMetrics } from "./metrics"
import {
  addClips,
  changeClips,
  currentSnapTicks,
  deleteClips,
  openPattern,
  setClipsMuted,
} from "./ops"
import { NO_DRAG, type ClipPainter } from "./painter"
import { PlaylistScene } from "./scene"
import { brushPattern, selectedClips } from "./selectors"
import { usePlaylistStore } from "./store"
import type { GridSurface } from "./surface"

/** A pointer event, already placed inside the grid. */
export type PointerInput = {
  /** CSS pixels from the grid's top left corner. */
  x: number
  y: number
  button: number
  shift: boolean
  /** Ctrl, or Cmd on macOS. */
  mod: boolean
  alt: boolean
}

type Point = { tick: number; row: number }

type Gesture =
  | { kind: "idle" }
  | { kind: "pan"; x: number; y: number }
  | {
      kind: "marquee"
      from: Point
      base: ReadonlySet<ClipId>
      ids: Set<ClipId>
    }
  | {
      kind: "move"
      anchor: Clip
      clips: Clip[]
      from: Point
      x: number
      y: number
      dragging: boolean
      /** The clip was selected before the press, so a Shift+click drops it. */
      wasSelected: boolean
      additive: boolean
    }
  | {
      kind: "resize"
      edge: "start" | "end"
      anchor: Clip
      clips: Clip[]
      x: number
      dragging: boolean
    }
  | { kind: "place"; pattern: Pattern }
  | { kind: "paint"; pattern: Pattern; anchor: number; row: number }
  | {
      kind: "stroke"
      action: "erase" | "mute"
      /** What a mute stroke sets: the opposite of the first clip it touches. */
      muteTo: boolean | null
      last: Point
    }
  /** The button is up and the edit is on its way to the project. */
  | { kind: "committing" }

const ui = () => usePlaylistStore.getState()
const NOTHING: ReadonlySet<ClipId> = new Set()

const plural = (count: number, one: string) => (count === 1 ? one : `${one}s`)

/**
 * Turns the pointer on the playlist's grid into edits. It talks to the
 * canvas through `GridSurface` and takes pointer input as plain numbers, so
 * all of it runs without a canvas or a DOM.
 *
 * Nothing is sent to the project while a button is down. A drag shows its
 * result by offsetting the selection on the canvas and by drawing the clips
 * to come on the overlay; the edit goes out once, when the button comes up.
 */
export class PlaylistSession {
  /** Called when the mouse cursor over the grid should change. */
  onCursor: (cursor: string) => void = () => {}

  private readonly surface: GridSurface
  private readonly metrics: GridMetrics
  private readonly scene: PlaylistScene
  private readonly now: () => number
  private gesture: Gesture = { kind: "idle" }
  private lastPress: { id: ClipId; time: number } | null = null
  private pressedAt: { x: number; y: number } | null = null
  private lastInput: PointerInput | null = null
  private scrollFrame = 0

  constructor(
    surface: GridSurface,
    metrics: GridMetrics,
    now: () => number = () => performance.now()
  ) {
    this.surface = surface
    this.metrics = metrics
    this.now = now
    this.scene = new PlaylistScene(surface, metrics)
    // The edit a drop sent has arrived, so what the drag was showing is now
    // the real thing. Dropping the preview in the same turn as the canvas
    // is rebuilt keeps the picture from jumping.
    this.scene.onClipsChanged = () => {
      if (this.gesture.kind === "committing") this.scene.clearPreview()
    }
  }

  destroy(): void {
    this.stopEdgeScroll()
    this.scene.destroy()
  }

  /** True while a button is down on the grid or an edit is being sent. */
  get busy(): boolean {
    return this.gesture.kind !== "idle"
  }

  /** The clips that will exist once the button is released. */
  get ghosts(): readonly NewClip[] {
    return this.scene.ghosts
  }

  /** The clips an erase or mute stroke has crossed so far. */
  get marked(): ReadonlySet<ClipId> {
    return this.scene.marked
  }

  get painter(): ClipPainter {
    return this.scene.painter
  }

  private settle(): void {
    this.gesture = { kind: "idle" }
    this.stopEdgeScroll()
    this.scene.clearPreview()
  }

  private pointAt(input: PointerInput): Point {
    const viewport = this.surface.viewport
    return {
      tick: xToTick(viewport, input.x),
      row: yToRow(viewport, input.y),
    }
  }

  private rowAt(point: Point): number {
    return clamp(Math.floor(point.row), 0, this.surface.limits.rowCount - 1)
  }

  private snapFor(input: PointerInput): number {
    // Alt lets go of the grid for one drag.
    return input.alt ? 0 : currentSnapTicks()
  }

  /** A button went down. Returns true when the drag should keep the pointer. */
  pointerDown(input: PointerInput): boolean {
    if (this.gesture.kind !== "idle") return false
    this.pressedAt = { x: input.x, y: input.y }
    const { tool, selection } = ui()
    const scene = this.scene
    const hit = scene.hitAt(input.x, input.y)
    const intent = intentFor({
      tool,
      button: input.button,
      mod: input.mod,
      shift: input.shift,
      hit,
    })
    const point = this.pointAt(input)

    const pressed =
      intent.kind === "move" ||
      intent.kind === "trim-start" ||
      intent.kind === "resize-end"
        ? scene.clipById.get(intent.id)
        : undefined
    if (pressed && this.isDoubleClick(pressed.id)) {
      void openPattern(pressed.content.pattern)
      return false
    }
    if (!pressed) this.lastPress = null

    switch (intent.kind) {
      case "none":
        return false
      case "pan":
        this.gesture = { kind: "pan", x: input.x, y: input.y }
        this.onCursor("grabbing")
        return true
      case "menu": {
        if (intent.id !== null && !selection.has(intent.id)) {
          ui().select([intent.id])
        }
        ui().setMenuOnClips(intent.id !== null)
        return false
      }
      case "marquee": {
        const base = intent.additive ? selection : NOTHING
        if (!intent.additive) ui().clearSelection()
        this.gesture = {
          kind: "marquee",
          from: point,
          base,
          ids: new Set(base),
        }
        return true
      }
      case "move": {
        if (!pressed) return false
        const wasSelected = selection.has(pressed.id)
        if (!wasSelected) {
          ui().select(
            intent.additive ? [...selection, pressed.id] : [pressed.id]
          )
        }
        this.gesture = {
          kind: "move",
          anchor: pressed,
          clips: selectedClips(),
          from: { tick: point.tick, row: Math.floor(point.row) },
          x: input.x,
          y: input.y,
          dragging: false,
          wasSelected,
          additive: intent.additive,
        }
        return true
      }
      case "trim-start":
      case "resize-end": {
        if (!pressed) return false
        if (!selection.has(pressed.id)) ui().select([pressed.id])
        this.gesture = {
          kind: "resize",
          edge: intent.kind === "trim-start" ? "start" : "end",
          anchor: pressed,
          clips: selectedClips(),
          x: input.x,
          dragging: false,
        }
        return true
      }
      case "place": {
        const pattern = brushPattern()
        if (!pattern) return false
        ui().clearSelection()
        this.gesture = { kind: "place", pattern }
        this.pointerMove(input)
        return true
      }
      case "paint": {
        const pattern = brushPattern()
        if (!pattern) return false
        ui().clearSelection()
        this.gesture = {
          kind: "paint",
          pattern,
          anchor: snapCell(point.tick, this.snapFor(input)),
          row: this.rowAt(point),
        }
        this.pointerMove(input)
        return true
      }
      case "erase":
      case "mute": {
        // The clip under the press counts even when it is a sliver that
        // the stroke's own box would miss.
        const first = hit ? scene.clipById.get(hit.id) : undefined
        this.gesture = {
          kind: "stroke",
          action: intent.kind,
          muteTo: first ? !first.muted : null,
          last: point,
        }
        scene.setMarked(new Set(first ? [first.id] : []))
        this.pointerMove(input)
        return true
      }
      default: {
        const _exhaustive: never = intent
        return _exhaustive
      }
    }
  }

  private isDoubleClick(id: ClipId): boolean {
    const time = this.now()
    const last = this.lastPress
    if (last && last.id === id && time - last.time <= DOUBLE_CLICK_MS) {
      this.lastPress = null
      return true
    }
    this.lastPress = { id, time }
    return false
  }

  /** The pointer moved, with or without a button down. */
  pointerMove(input: PointerInput): void {
    const gesture = this.gesture
    const scene = this.scene
    const point = this.pointAt(input)
    switch (gesture.kind) {
      case "idle": {
        const hit = scene.hitAt(input.x, input.y)
        this.onCursor(cursorFor(ui().tool, hit?.part ?? null))
        return
      }
      case "committing":
        return
      case "pan":
        this.metrics.panBy(gesture.x - input.x, gesture.y - input.y)
        gesture.x = input.x
        gesture.y = input.y
        return
      case "marquee": {
        const items = scene.items
        if (!items) return
        scene.setMarquee({
          tick0: gesture.from.tick,
          row0: gesture.from.row,
          tick1: point.tick,
          row1: point.row,
        })
        // Padded like a stroke, so a box dragged dead level still has area.
        const box = strokeBox(gesture.from, point)
        const ids = new Set(gesture.base)
        for (const index of queryRect(
          items,
          box.tick0,
          box.tick1,
          box.row0,
          box.row1
        )) {
          ids.add(items.batch.ids[index])
        }
        gesture.ids = ids
        // The store hears about it once, when the button comes up.
        scene.showSelected(ids)
        break
      }
      case "move": {
        if (!gesture.dragging) {
          const far =
            Math.hypot(input.x - gesture.x, input.y - gesture.y) >=
            DRAG_THRESHOLD_PX
          if (!far) return
          gesture.dragging = true
          this.onCursor("grabbing")
        }
        const move = clampMove(
          gesture.clips.map((clip) => ({
            start: clip.start,
            row: scene.rowOf(clip.track),
          })),
          gesture.anchor.start,
          point.tick - gesture.from.tick,
          Math.floor(point.row) - gesture.from.row,
          this.snapFor(input),
          this.surface.limits.rowCount
        )
        scene.cloneHint = input.shift
        scene.setDrag({ ...NO_DRAG, ticks: move.ticks, rows: move.rows })
        break
      }
      case "resize": {
        if (!gesture.dragging) {
          if (Math.abs(input.x - gesture.x) < DRAG_THRESHOLD_PX) return
          gesture.dragging = true
        }
        const snap = this.snapFor(input)
        const minLength = minResizeLength(snap)
        scene.setDrag(
          gesture.edge === "end"
            ? {
                ...NO_DRAG,
                resizeEnd: endResizeDelta(gesture.anchor, point.tick, snap),
                minLength,
              }
            : {
                ...NO_DRAG,
                resizeStart: startTrimDelta(
                  gesture.clips,
                  gesture.anchor,
                  point.tick,
                  snap
                ),
                minLength,
              }
        )
        break
      }
      case "place":
        scene.setGhosts([
          this.brushClip(
            gesture.pattern,
            snapCell(point.tick, this.snapFor(input)),
            this.rowAt(point)
          ),
        ])
        break
      case "paint": {
        const pass = patternTicks(gesture.pattern)
        scene.setGhosts(
          withoutStacked(
            paintStarts(gesture.anchor, point.tick, pass).map((start) =>
              this.brushClip(gesture.pattern, start, gesture.row)
            ),
            scene.clips,
            scene.rowOf
          )
        )
        break
      }
      case "stroke": {
        const items = scene.items
        if (!items) return
        const box = strokeBox(gesture.last, point)
        gesture.last = point
        const marked = new Set(scene.marked)
        for (const index of queryRect(
          items,
          box.tick0,
          box.tick1,
          box.row0,
          box.row1
        )) {
          const clip = scene.clipById.get(items.batch.ids[index])
          if (!clip) continue
          gesture.muteTo ??= !clip.muted
          marked.add(clip.id)
        }
        if (marked.size !== scene.marked.size) scene.setMarked(marked)
        break
      }
      default: {
        const _exhaustive: never = gesture
        return _exhaustive
      }
    }
    this.edgeScroll(input)
  }

  private brushClip(pattern: Pattern, start: number, row: number): NewClip {
    return {
      row,
      start,
      length: patternTicks(pattern),
      offset: 0,
      muted: false,
      pattern: pattern.id,
    }
  }

  /**
   * Scrolls while a drag is held at an edge of the grid, so a clip can be
   * taken to a bar or a track that is out of view. Each frame it scrolls a
   * little and moves the pointer again, which looks at the edge again.
   */
  private edgeScroll(input: PointerInput): void {
    // A press near an edge is not a drag toward it until it has moved.
    const from = this.pressedAt
    if (
      from &&
      Math.hypot(input.x - from.x, input.y - from.y) < DRAG_THRESHOLD_PX
    ) {
      return
    }
    this.pressedAt = null
    this.lastInput = input
    if (this.scrollFrame !== 0) return
    const { width, height } = this.surface.viewport
    if (edgePull(input.x, width) === 0 && edgePull(input.y, height) === 0) {
      return
    }
    this.scrollFrame = requestAnimationFrame(() => {
      this.scrollFrame = 0
      const held = this.lastInput
      const kind = this.gesture.kind
      if (!held || kind === "idle" || kind === "committing" || kind === "pan") {
        return
      }
      const before = this.surface.viewport
      this.metrics.panBy(
        edgePull(held.x, before.width),
        edgePull(held.y, before.height)
      )
      // At the end of the timeline or the rows there is nowhere to go.
      if (this.surface.viewport !== before) this.pointerMove(held)
    })
  }

  private stopEdgeScroll(): void {
    if (this.scrollFrame !== 0) cancelAnimationFrame(this.scrollFrame)
    this.scrollFrame = 0
    this.lastInput = null
  }

  /** The button came up. Resolves once the edit, if any, has been applied. */
  async pointerUp(input: PointerInput): Promise<void> {
    const gesture = this.gesture
    const scene = this.scene
    this.stopEdgeScroll()
    switch (gesture.kind) {
      case "idle":
      case "committing":
        return
      case "pan":
        this.gesture = { kind: "idle" }
        this.pointerMove(input)
        return
      case "marquee":
        ui().select(gesture.ids)
        this.settle()
        return
      case "move": {
        if (!gesture.dragging) {
          const id = gesture.anchor.id
          if (gesture.additive && gesture.wasSelected) {
            ui().select([...ui().selection].filter((other) => other !== id))
          } else if (!gesture.additive) {
            ui().select([id])
          }
          this.settle()
          return
        }
        const { ticks, rows } = scene.drag
        if (ticks === 0 && rows === 0) {
          this.settle()
          return
        }
        this.gesture = { kind: "committing" }
        if (input.shift) {
          // The originals stay, so they go back and the copies show as
          // clips to come until they arrive.
          const copies = cloneMoved(gesture.clips, scene.rowOf, ticks, rows)
          scene.cloneHint = false
          scene.setDrag(NO_DRAG)
          scene.setGhosts(copies)
          const created = await addClips(
            copies,
            plural(copies.length, "Clone clip")
          )
          if (created && created.length > 0) ui().select(created)
        } else {
          await changeClips(
            moveChanges(gesture.clips, scene.rowOf, ticks, rows),
            plural(gesture.clips.length, "Move clip")
          )
        }
        this.settle()
        return
      }
      case "resize": {
        const changes = gesture.dragging
          ? resizeChanges(
              gesture.clips,
              scene.drag.resizeStart,
              scene.drag.resizeEnd,
              scene.drag.minLength,
              scene.passTicks
            )
          : []
        if (changes.length === 0) {
          this.settle()
          return
        }
        this.gesture = { kind: "committing" }
        await changeClips(
          changes,
          plural(
            changes.length,
            gesture.edge === "end" ? "Resize clip" : "Trim clip"
          )
        )
        this.settle()
        return
      }
      case "place":
      case "paint": {
        const clips = scene.ghosts
        this.gesture = { kind: "committing" }
        await addClips(
          clips,
          gesture.kind === "place"
            ? "Add clip"
            : plural(clips.length, "Paint clip")
        )
        this.settle()
        return
      }
      case "stroke": {
        const ids = [...scene.marked]
        if (ids.length === 0) {
          this.settle()
          return
        }
        this.gesture = { kind: "committing" }
        if (gesture.action === "erase") await deleteClips(ids)
        else await setClipsMuted(ids, gesture.muteTo ?? true)
        this.settle()
        return
      }
      default: {
        const _exhaustive: never = gesture
        return _exhaustive
      }
    }
  }

  /** Drops a drag without doing anything: Escape, or a lost pointer. */
  cancel(): void {
    if (this.gesture.kind === "committing") return
    if (this.gesture.kind === "marquee") this.scene.showSelection()
    this.settle()
  }
}
