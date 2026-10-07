import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ClipId, PlaylistTrackId } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"

import type { NewClip } from "./edit"
import type { Tool } from "./intents"
import type { SnapMode } from "./snap"

const NO_CLIPS: ReadonlySet<ClipId> = new Set()

type PlaylistUiState = {
  tool: Tool
  snap: SnapMode
  /** Keep the playhead in view while the song plays. */
  follow: boolean
  pickerOpen: boolean
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

  setTool(tool: Tool): void
  setSnap(snap: SnapMode): void
  toggleFollow(): void
  togglePicker(): void
  select(ids: Iterable<ClipId>): void
  clearSelection(): void
  setTargetTrack(track: PlaylistTrackId | null): void
  setCursorTick(tick: number): void
  setClipboard(clips: readonly NewClip[]): void
  setMenuOnClips(onClips: boolean): void
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
      selection: NO_CLIPS,
      targetTrack: null,
      cursorTick: 0,
      clipboard: [],
      menuOnClips: false,

      setTool: (tool) => set({ tool }),
      setSnap: (snap) => set({ snap }),
      toggleFollow: () => set((state) => ({ follow: !state.follow })),
      togglePicker: () => set((state) => ({ pickerOpen: !state.pickerOpen })),
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
    }),
    {
      name: "windfall.playlist",
      version: 1,
      partialize: (state) => ({
        tool: state.tool,
        snap: state.snap,
        follow: state.follow,
        pickerOpen: state.pickerOpen,
      }),
    }
  )
)

// Ids start over in every project. A revision that goes back means another
// project was loaded, so whatever points into the old one is dropped: a
// clip with the same id in the new project is not the clip that was selected.
useProjectStore.subscribe((state, previous) => {
  if (state.revision >= previous.revision) return
  usePlaylistStore.setState({
    selection: NO_CLIPS,
    targetTrack: null,
    cursorTick: 0,
    clipboard: [],
    menuOnClips: false,
  })
})
