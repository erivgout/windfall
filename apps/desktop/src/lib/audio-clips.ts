import type {
  BrowserRoot,
  Project,
  SampleAsset,
  SampleId,
  TrackId,
} from "@/bindings"

import { MAX_AUDIO_CLIPS } from "./units"

/*
 * Rules about audio clips that the playlist and the mock backend both go
 * by, so a clip lands the same whichever side decides.
 */

/**
 * The mixer track a new clip of a sample plays into when none is asked
 * for: the one the most recent clip of that sample uses, so three drops of
 * one file share a track instead of making three. Undefined when no clip
 * plays the sample yet, or the track is gone, which makes a track for it.
 */
export function mixerTrackOfSample(
  project: Pick<Project, "mixer" | "playlist">,
  sample: SampleId
): TrackId | undefined {
  const tracks = new Set(project.mixer.tracks.map((track) => track.id))
  let found: { id: number; track: TrackId } | undefined
  for (const clip of project.playlist.clips) {
    const content = clip.content
    if (content.type !== "audio" || content.sample !== sample) continue
    if (!tracks.has(content.mixerTrack)) continue
    // Ids only ever go up, so the highest is the clip made last.
    if (!found || clip.id > found.id) {
      found = { id: clip.id, track: content.mixerTrack }
    }
  }
  return found?.track
}

const slashes = (path: string) => path.replace(/\\/g, "/").replace(/\/+$/, "")

/** `file` as a path inside `folder`, or null when it is not in there. */
function inside(folder: string, file: string): string | null {
  const base = slashes(folder)
  const path = slashes(file)
  return path.startsWith(`${base}/`) ? path.slice(base.length + 1) : null
}

function folderOf(file: string): string {
  const path = slashes(file)
  const cut = path.lastIndexOf("/")
  return cut < 0 ? "" : path.slice(0, cut)
}

/**
 * The sample of the project's pool that is this file, if the project has
 * it: a factory sound by its place in a factory folder, a sample copied
 * into the project by its place beside the project file, and any other by
 * its whole path.
 */
export function sampleOfFile(
  samples: readonly SampleAsset[],
  file: string,
  where: { roots: readonly BrowserRoot[]; projectPath: string | null }
): SampleAsset | undefined {
  const path = slashes(file)
  const factory = where.roots
    .filter((root) => root.kind === "factory")
    .map((root) => inside(root.path, file))
    .filter((relative) => relative !== null)
  const inProject =
    where.projectPath === null
      ? null
      : inside(folderOf(where.projectPath), file)
  return samples.find((sample) => {
    const stored = slashes(sample.path.path)
    switch (sample.path.kind) {
      case "factory":
        return factory.includes(stored)
      case "project":
        return inProject === stored
      case "external":
        return stored === path
      default: {
        const _exhaustive: never = sample.path
        return _exhaustive
      }
    }
  })
}

/** Where a song asks for more audio clips at once than can sound. */
export type AudioClipOverlap = {
  /** The most audio clips that sound at one time anywhere in the song. */
  most: number
  /**
   * The first tick at which more than the limit sound together, or null
   * when that never happens.
   */
  overAt: number | null
}

/**
 * Counts how many audio clips sound together across the song, from where
 * the clips lie: the unmuted audio clips on unmuted playlist tracks, each
 * from its start to its end. A clip whose audio runs out before its end
 * still counts to the end, so this is never too low.
 *
 * It is an estimate made from the project. The engine knows which clips it
 * really could not start; where it says so, that count is the one to show.
 */
export function audioClipOverlap(
  playlist: Pick<Project["playlist"], "clips" | "tracks">,
  limit = MAX_AUDIO_CLIPS
): AudioClipOverlap {
  const silent = new Set(
    playlist.tracks.filter((track) => track.muted).map((track) => track.id)
  )
  // +1 where a clip starts and -1 where it ends, in song order. An end
  // comes before a start on the same tick: back to back is not together.
  const edges: [tick: number, change: number][] = []
  for (const clip of playlist.clips) {
    if (clip.content.type !== "audio" || clip.muted) continue
    if (silent.has(clip.track)) continue
    edges.push([clip.start, 1], [clip.start + clip.length, -1])
  }
  edges.sort((a, b) => a[0] - b[0] || a[1] - b[1])
  let sounding = 0
  let most = 0
  let overAt: number | null = null
  for (const [tick, change] of edges) {
    sounding += change
    if (sounding > most) most = sounding
    if (overAt === null && sounding > limit) overAt = tick
  }
  return { most, overAt }
}
