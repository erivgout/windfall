import type {
  AudioClipPatch,
  AudioClipUpdate,
  ClipStretch,
  ClipUpdate,
  Command,
} from "@/bindings"
import { registry, type Action } from "@/lib/actions"
import { dispatch, useProjectStore } from "@/lib/store/project"

async function setAudioClipStretch(stretch: ClipStretch): Promise<void> {
  const updates: AudioClipUpdate[] = []
  for (const clip of useProjectStore.getState().project.playlist.clips) {
    if (clip.content.type !== "audio") continue
    const current = clip.content.stretch ?? { mode: "tape" }
    const unchanged =
      stretch.mode === "tape"
        ? current.mode === "tape"
        : current.mode === "spectral" &&
          current.ratio === stretch.ratio &&
          current.quality === stretch.quality &&
          current.formants === stretch.formants
    if (!unchanged) updates.push({ id: clip.id, patch: { stretch } })
  }
  if (updates.length > 0) await dispatch({ type: "updateAudioClips", updates })
}

async function muteEmptyPlaylistTracks(): Promise<void> {
  const { tracks, clips } = useProjectStore.getState().project.playlist
  const occupied = new Set(clips.map((clip) => clip.track))
  const commands: Command[] = tracks
    .filter((track) => !track.muted && !occupied.has(track.id))
    .map((track) => ({
      type: "updatePlaylistTrack",
      id: track.id,
      patch: { muted: true },
    }))
  if (commands.length > 0) {
    await dispatch({
      type: "batch",
      label: "Mute empty playlist tracks",
      commands,
    })
  }
}

async function unsoloPlaylistTracks(): Promise<void> {
  const commands: Command[] = useProjectStore
    .getState()
    .project.playlist.tracks.filter((track) => track.solo === true)
    .map((track) => ({
      type: "updatePlaylistTrack",
      id: track.id,
      patch: { solo: false },
    }))
  if (commands.length > 0) {
    await dispatch({
      type: "batch",
      label: "Unsolo playlist tracks",
      commands,
    })
  }
}

async function resetAudioClipLevels(): Promise<void> {
  const updates: AudioClipUpdate[] = []
  for (const clip of useProjectStore.getState().project.playlist.clips) {
    if (clip.content.type !== "audio") continue
    const patch: AudioClipPatch = {}
    if (clip.content.gain !== 1) patch.gain = 1
    if (clip.content.pan !== 0) patch.pan = 0
    if (Object.keys(patch).length > 0) updates.push({ id: clip.id, patch })
  }
  if (updates.length > 0) await dispatch({ type: "updateAudioClips", updates })
}

async function unmutePlaylistClips(): Promise<void> {
  const updates: ClipUpdate[] = []
  for (const clip of useProjectStore.getState().project.playlist.clips) {
    if (clip.muted === true) updates.push({ id: clip.id, patch: { muted: false } })
  }
  if (updates.length > 0) await dispatch({ type: "updateClips", updates })
}

const TOOL_ACTIONS: Action[] = [
  {
    id: "tools.setAllAudioClipsToTape",
    title: "Set all audio clips to tape",
    section: "Tools",
    run: () => setAudioClipStretch({ mode: "tape" }),
  },
  {
    id: "tools.setAllAudioClipsToSpectral",
    title: "Set all audio clips to spectral",
    section: "Tools",
    run: () =>
      setAudioClipStretch({
        mode: "spectral",
        ratio: 1,
        quality: "standard",
        formants: false,
      }),
  },
  {
    id: "tools.muteEmptyPlaylistTracks",
    title: "Mute empty playlist tracks",
    section: "Tools",
    run: muteEmptyPlaylistTracks,
  },
  {
    id: "tools.unsoloPlaylistTracks",
    title: "Unsolo playlist tracks",
    section: "Tools",
    run: unsoloPlaylistTracks,
  },
  {
    id: "tools.resetAudioClipLevels",
    title: "Reset audio clip levels",
    section: "Tools",
    run: resetAudioClipLevels,
  },
  {
    id: "tools.unmutePlaylistClips",
    title: "Unmute playlist clips",
    section: "Tools",
    run: unmutePlaylistClips,
  },
]

export function registerToolsActions(): () => void {
  return registry.register(TOOL_ACTIONS)
}
