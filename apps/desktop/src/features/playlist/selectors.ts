import { useShallow } from "zustand/react/shallow"

import type {
  Automation,
  AutomationId,
  Clip,
  Pattern,
  PatternId,
  PlaylistTrack,
  Project,
  SampleAsset,
  SampleId,
} from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { selectedPatternId } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"

import { usePlaylistStore } from "./store"

/* Reads of the project the playlist needs, as hooks and as plain calls. */

export const project = (): Project => useProjectStore.getState().project
export const playlist = () => project().playlist

export function usePlaylistTracks(): PlaylistTrack[] {
  return useProjectStore((state) => state.project.playlist.tracks)
}

export function useClipCount(): number {
  return useProjectStore((state) => state.project.playlist.clips.length)
}

/** How many clips play each pattern, in the order of `ids`. */
export function useClipCounts(ids: readonly PatternId[]): number[] {
  return useProjectStore(
    useShallow((state) => {
      const counts = new Map<PatternId, number>()
      for (const clip of state.project.playlist.clips) {
        if (clip.content.type !== "pattern") continue
        const pattern = clip.content.pattern
        counts.set(pattern, (counts.get(pattern) ?? 0) + 1)
      }
      return ids.map((id) => counts.get(id) ?? 0)
    })
  )
}

/** The pattern selected app-wide, which a pattern brush places. */
export function brushPattern(): Pattern | undefined {
  const current = project()
  const id = selectedPatternId(current, useTransportStore.getState().pattern)
  return current.patterns.find((pattern) => pattern.id === id)
}

/** What the Draw and Paint tools place, looked up in the project. */
export type ResolvedBrush =
  | { type: "pattern"; pattern: Pattern }
  | { type: "audio"; sample: SampleAsset }
  | { type: "automation"; automation: Automation }

/**
 * The brush, with what it places. A sample or an automation that has been
 * deleted since it was picked leaves the selected pattern as the brush.
 */
export function resolveBrush(): ResolvedBrush | null {
  const current = project()
  const brush = usePlaylistStore.getState().brush
  if (brush.type === "audio") {
    const sample = current.samples.find((item) => item.id === brush.sample)
    if (sample) return { type: "audio", sample }
  }
  if (brush.type === "automation") {
    const automation = current.automations.find(
      (item) => item.id === brush.automation
    )
    if (automation) return { type: "automation", automation }
  }
  const pattern = brushPattern()
  return pattern ? { type: "pattern", pattern } : null
}

/** How many clips play each sample, by sample id. */
export function useAudioClipCounts(): ReadonlyMap<SampleId, number> {
  const clips = useProjectStore((state) => state.project.playlist.clips)
  const counts = new Map<SampleId, number>()
  for (const clip of clips) {
    if (clip.content.type !== "audio") continue
    counts.set(clip.content.sample, (counts.get(clip.content.sample) ?? 0) + 1)
  }
  return counts
}

/** How many clips show each automation, by automation id. */
export function useAutomationClipCounts(): ReadonlyMap<AutomationId, number> {
  const clips = useProjectStore((state) => state.project.playlist.clips)
  const counts = new Map<AutomationId, number>()
  for (const clip of clips) {
    if (clip.content.type !== "automation") continue
    const id = clip.content.automation
    counts.set(id, (counts.get(id) ?? 0) + 1)
  }
  return counts
}

/** The selected clips that still exist, in timeline order. */
export function selectedClips(): Clip[] {
  const { selection } = usePlaylistStore.getState()
  if (selection.size === 0) return []
  return playlist().clips.filter((clip) => selection.has(clip.id))
}
