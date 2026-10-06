import { registry, type Action, type AppState } from "@/lib/actions"
import { useUiStore } from "@/lib/store"
import { MASTER_TRACK, MAX_MIXER_TRACKS } from "@/lib/units"

import {
  addTrack,
  centerPan,
  deleteTrack,
  resetAllPeaks,
  resetVolume,
  selectStrip,
  setOutput,
  startColoring,
  startRename,
  toggleMute,
  toggleSolo,
  unmuteAll,
  unsoloAll,
} from "./operations"

const SECTION = "Mixer"

// The selection lives in the UI store, which the registry's state does not
// carry, so the selected track is looked up from the document it was given.
function selected(state: AppState) {
  const id = useUiStore.getState().selectedTrack
  return state.document.project.mixer.tracks.find((track) => track.id === id)
}

function selectedInsert(state: AppState) {
  const track = selected(state)
  return track && track.id !== MASTER_TRACK ? track : undefined
}

/** Runs `work` on the selected track. Does nothing when there is none. */
function onSelected(work: (id: number) => void | Promise<void>) {
  return () => {
    const id = useUiStore.getState().selectedTrack
    if (id !== null) return work(id)
  }
}

export const MIXER_ACTIONS: Action[] = [
  {
    id: "mixer.addTrack",
    title: "Add mixer track",
    section: SECTION,
    defaultShortcut: "Alt+M",
    keywords: "new insert bus",
    enabled: (state) =>
      state.document.project.mixer.tracks.length < MAX_MIXER_TRACKS,
    run: addTrack,
  },
  {
    id: "mixer.renameTrack",
    title: "Rename mixer track",
    section: SECTION,
    keywords: "name",
    enabled: (state) => selected(state) !== undefined,
    run: onSelected(startRename),
  },
  {
    id: "mixer.changeColor",
    title: "Change track color…",
    section: SECTION,
    keywords: "colour swatch",
    enabled: (state) => selected(state) !== undefined,
    run: onSelected(startColoring),
  },
  {
    id: "mixer.deleteTrack",
    title: "Delete mixer track",
    section: SECTION,
    keywords: "remove",
    enabled: (state) => selectedInsert(state) !== undefined,
    run: onSelected(deleteTrack),
  },
  {
    id: "mixer.toggleMute",
    title: "Mute or unmute track",
    section: SECTION,
    keywords: "silence",
    enabled: (state) => selected(state) !== undefined,
    checked: (state) => selected(state)?.muted ?? false,
    run: onSelected(toggleMute),
  },
  {
    id: "mixer.toggleSolo",
    title: "Solo or unsolo track",
    section: SECTION,
    keywords: "isolate",
    enabled: (state) => selectedInsert(state) !== undefined,
    checked: (state) => selectedInsert(state)?.solo ?? false,
    run: onSelected(toggleSolo),
  },
  {
    id: "mixer.unmuteAll",
    title: "Unmute all tracks",
    section: SECTION,
    enabled: (state) =>
      state.document.project.mixer.tracks.some((track) => track.muted),
    run: unmuteAll,
  },
  {
    id: "mixer.unsoloAll",
    title: "Unsolo all tracks",
    section: SECTION,
    enabled: (state) =>
      state.document.project.mixer.tracks.some((track) => track.solo),
    run: unsoloAll,
  },
  {
    id: "mixer.resetVolume",
    title: "Reset fader to 0 dB",
    section: SECTION,
    keywords: "volume level gain unity",
    enabled: (state) => (selected(state)?.volume ?? 1) !== 1,
    run: onSelected(resetVolume),
  },
  {
    id: "mixer.centerPan",
    title: "Center pan",
    section: SECTION,
    keywords: "reset balance",
    enabled: (state) => (selected(state)?.pan ?? 0) !== 0,
    run: onSelected(centerPan),
  },
  {
    id: "mixer.routeToMaster",
    title: "Route to master",
    section: SECTION,
    keywords: "output reset routing",
    enabled: (state) => {
      const track = selectedInsert(state)
      return track !== undefined && track.output !== MASTER_TRACK
    },
    run: onSelected((id) => setOutput(id, MASTER_TRACK)),
  },
  {
    id: "mixer.resetPeaks",
    title: "Reset peak readouts",
    section: SECTION,
    keywords: "clip clear meters",
    run: resetAllPeaks,
  },
  {
    id: "mixer.selectNext",
    title: "Select next mixer track",
    section: SECTION,
    run: () => {
      selectStrip(1)
    },
  },
  {
    id: "mixer.selectPrevious",
    title: "Select previous mixer track",
    section: SECTION,
    run: () => {
      selectStrip(-1)
    },
  },
]

let holders = 0
let unregister: (() => void) | null = null

/**
 * Puts the mixer's actions in the registry. The app registers them at boot
 * and the panel does too, so they are there whichever comes first; they
 * leave the registry when the last holder lets go.
 */
export function registerMixerActions(): () => void {
  holders += 1
  unregister ??= registry.register(MIXER_ACTIONS)
  let released = false
  return () => {
    if (released) return
    released = true
    holders -= 1
    if (holders === 0) {
      unregister?.()
      unregister = null
    }
  }
}
