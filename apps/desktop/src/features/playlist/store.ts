import { create } from "zustand"
import { persist } from "zustand/middleware"

import type {
  AutomationId,
  ClipId,
  PlaylistTrackId,
  SampleId,
} from "@/bindings"
import { onProjectReplaced } from "@/lib/store/replaced"

import type { NewClip } from "./edit"
import type { Tool } from "./intents"
import type { SnapMode } from "./snap"

const NO_CLIPS: ReadonlySet<ClipId> = new Set()

/**
 * What the Draw and Paint tools place. A pattern brush places the pattern
 * selected app-wide, so it carries no id of its own.
 */
export type Brush =
  | { type: "pattern" }
  | { type: "audio"; sample: SampleId }
  | { type: "automation"; automation: AutomationId }

const PATTERN_BRUSH: Brush = { type: "pattern" }

/** A point of a curve, as the clip it was pressed in shows it. */
export type PointRef = {
  clip: ClipId
  automation: AutomationId
  index: number
}

type PlaylistUiState = {
  tool: Tool
  snap: SnapMode
  /** Keep the playhead in view while the song plays. */
  follow: boolean
  pickerOpen: boolean
  /** Show the settings of the selected audio clips above the timeline. */
  inspectorOpen: boolean
  brush: Brush
  selection: ReadonlySet<ClipId>
  /** The track the track actions act on: the header last pressed. */
  targetTrack: PlaylistTrackId | null
  /**
   * Where the song will play from. In song mode it follows the playhead; in
   * pattern mode the engine keeps no song position, so the ruler sets it.
   */
  cursorTick: number
  /** Copied clips, with times counted from the earliest one. */
  clipboard: readonly NewClip[]
  /** What the grid's right-click menu was opened on. */
  menuOnClips: boolean
  /** The point of a curve the menu was opened on, if it was on one. */
  menuPoint: PointRef | null
  /** A clip the timeline should scroll to, once it is on screen. */
  reveal: ClipId | null
  /**
   * The timeline should take the keyboard, once it is on screen: what was
   * just selected in it is what the next key is meant for.
   */
  focusRequested: boolean

  setTool(tool: Tool): void
  setSnap(snap: SnapMode): void
  toggleFollow(): void
  togglePicker(): void
  toggleInspector(): void
  setBrush(brush: Brush): void
  select(ids: Iterable<ClipId>): void
  clearSelection(): void
  setTargetTrack(track: PlaylistTrackId | null): void
  setCursorTick(tick: number): void
  setClipboard(clips: readonly NewClip[]): void
  setMenuOnClips(onClips: boolean): void
  setMenuPoint(point: PointRef | null): void
  setReveal(clip: ClipId | null): void
  requestFocus(): void
  focusGiven(): void
}

/**
 * View state of the playlist that is not part of the project. The tool, the
 * snap and the layout choices survive a restart; the selection does not.
 */
export const usePlaylistStore = create<PlaylistUiState>()(
  persist(
    (set, get) => ({
      tool: "draw",
      snap: "bar",
      follow: true,
      pickerOpen: true,
      inspectorOpen: true,
      brush: PATTERN_BRUSH,
      selection: NO_CLIPS,
      targetTrack: null,
      cursorTick: 0,
      clipboard: [],
      menuOnClips: false,
      menuPoint: null,
      reveal: null,
      focusRequested: false,

      setTool: (tool) => set({ tool }),
      setSnap: (snap) => set({ snap }),
      toggleFollow: () => set((state) => ({ follow: !state.follow })),
      togglePicker: () => set((state) => ({ pickerOpen: !state.pickerOpen })),
      toggleInspector: () =>
        set((state) => ({ inspectorOpen: !state.inspectorOpen })),
      setBrush: (brush) => {
        const current = get().brush
        const same =
          current.type === brush.type &&
          (brush.type !== "audio" ||
            (current.type === "audio" && current.sample === brush.sample)) &&
          (brush.type !== "automation" ||
            (current.type === "automation" &&
              current.automation === brush.automation))
        if (!same)
          set({ brush: brush.type === "pattern" ? PATTERN_BRUSH : brush })
      },
      select: (ids) => {
        const next = new Set(ids)
        const current = get().selection
        // Keeps the same set when nothing changed, so nothing redraws.
        if (
          next.size === current.size &&
          [...next].every((id) => current.has(id))
        ) {
          return
        }
        set({ selection: next.size === 0 ? NO_CLIPS : next })
      },
      clearSelection: () => {
        if (get().selection.size > 0) set({ selection: NO_CLIPS })
      },
      setTargetTrack: (targetTrack) => set({ targetTrack }),
      setCursorTick: (tick) => {
        const cursorTick = Math.max(0, tick)
        if (cursorTick !== get().cursorTick) set({ cursorTick })
      },
      setClipboard: (clipboard) => set({ clipboard }),
      setMenuOnClips: (menuOnClips) => {
        if (menuOnClips !== get().menuOnClips) set({ menuOnClips })
      },
      setMenuPoint: (menuPoint) => {
        if (menuPoint !== get().menuPoint) set({ menuPoint })
      },
      setReveal: (reveal) => {
        if (reveal !== get().reveal) set({ reveal })
      },
      requestFocus: () => set({ focusRequested: true }),
      focusGiven: () => {
        if (get().focusRequested) set({ focusRequested: false })
      },
    }),
    {
      name: "windfall.playlist",
      version: 1,
      partialize: (state) => ({
        tool: state.tool,
        snap: state.snap,
        follow: state.follow,
        pickerOpen: state.pickerOpen,
        inspectorOpen: state.inspectorOpen,
      }),
    }
  )
)

// Ids start over in every project, so whatever points into the old one is
// dropped: a clip with the same id in the new project is not the clip that
// was selected.
onProjectReplaced(() =>
  usePlaylistStore.setState({
    selection: NO_CLIPS,
    targetTrack: null,
    cursorTick: 0,
    clipboard: [],
    menuOnClips: false,
    menuPoint: null,
    reveal: null,
    brush: PATTERN_BRUSH,
  })
)
