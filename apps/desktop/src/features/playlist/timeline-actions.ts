import type { Action, AppState } from "@/lib/actions"
import { backend } from "@/lib/ipc"
import { useUiStore } from "@/lib/store/ui"
import { activeMetrics } from "./active"
import { songTick } from "./ops"
import { usePlaylistStore } from "./store"
import {
  beginTimelineRequest,
  finishTimelinePlay,
  publishPlaybackRegion,
  timelineRequestError,
  editTimeline,
  selectTimelineRegion,
  useTimelineStore,
} from "./timeline-store"

const timeline = () => useTimelineStore.getState()
const inPlaylist = (state: AppState) => state.ui.centerTab === "playlist"
const hasRange = (state: AppState) =>
  inPlaylist(state) && !!timeline().selection

export async function playTimelineSelection(loopSong: boolean) {
  const range = timeline().selection
  if (!range) return
  const operation = beginTimelineRequest(true)
  const guard = await publishPlaybackRegion({ ...range }, operation)
  if (!guard || !operation.current()) return
  try {
    await backend.transportSet({ mode: "song", loopSong }, guard)
    if (!operation.current()) return
    await backend.transportSeek(range.start, guard)
    if (!operation.current()) return
    usePlaylistStore.getState().setCursorTick(range.start)
    await backend.transportPlay(guard)
    if (!operation.current()) return
    finishTimelinePlay(operation)
  } catch (error) {
    timelineRequestError(operation, error)
  }
}

export function exportTimelineSelection() {
  if (!timeline().selection) return
  useTimelineStore.setState({ exportSelection: true })
  useUiStore.getState().openDialog("export")
}

export async function clearTimelineSelection() {
  await selectTimelineRegion(null)
}

export const TIMELINE_ACTIONS: Action[] = [
  ...(["seek", "select", "zoom"] as const).map((tool): Action => ({
    id: `playlist.ruler${tool}`,
    title: {
      seek: "Seek with the ruler",
      select: "Select time with the ruler",
      zoom: "Zoom a dragged region",
    }[tool],
    section: "Playlist",
    keywords: "timeline region ruler",
    enabled: inPlaylist,
    checked: () => timeline().tool === tool,
    run: () => useTimelineStore.setState({ tool }),
  })),
  {
    id: "playlist.addMeterChange",
    title: "Add song meter change…",
    section: "Playlist",
    keywords: "time signature bar beat timeline",
    enabled: inPlaylist,
    run: () => editTimeline({ type: "meter" }),
  },
  ...(["named", "loop", "skip", "pause"] as const).map((type): Action => ({
    id: `playlist.add${type}Marker`,
    title: `Add ${type} timeline marker…`,
    section: "Playlist",
    keywords: "timeline navigation label",
    enabled: inPlaylist,
    run: () =>
      editTimeline({
        type: "marker",
        kind:
          type === "loop" || type === "skip"
            ? {
                type,
                end: timeline().selection?.end ?? Math.floor(songTick()) + 3840,
              }
            : { type },
      }),
  })),
  ...([false, true] as const).map((loopSong): Action => ({
    id: loopSong ? "playlist.loopSelection" : "playlist.playSelection",
    title: loopSong ? "Loop selected song region" : "Play selected song region",
    section: "Playlist",
    keywords: "timeline time selection range",
    enabled: hasRange,
    whyDisabled: (state) =>
      inPlaylist(state)
        ? "Select time in the playlist ruler first"
        : "Open the playlist first",
    run: () => playTimelineSelection(loopSong),
  })),
  {
    id: "playlist.zoomRegion",
    title: "Zoom to selected song region",
    section: "Playlist",
    enabled: (state) => hasRange(state) && activeMetrics() !== null,
    whyDisabled: (state) =>
      !inPlaylist(state)
        ? "Open the playlist first"
        : timeline().selection
          ? "Open the playlist canvas first"
          : "Select time in the playlist ruler first",
    run: () => {
      const range = timeline().selection
      if (range) activeMetrics()?.fitRegion(range)
    },
  },
  {
    id: "playlist.exportRegion",
    title: "Export selected song region…",
    section: "Playlist",
    enabled: hasRange,
    whyDisabled: (state) =>
      inPlaylist(state)
        ? "Select time in the playlist ruler first"
        : "Open the playlist first",
    run: exportTimelineSelection,
  },
  {
    id: "playlist.clearRegion",
    title: "Clear song time selection",
    section: "Playlist",
    enabled: (state) =>
      inPlaylist(state) &&
      (!!timeline().selection || timeline().active || !timeline().hydrated),
    whyDisabled: (state) =>
      inPlaylist(state)
        ? "No song time selection to clear"
        : "Open the playlist first",
    run: clearTimelineSelection,
  },
]
