import type { HistoryView, PatternId, Project, ProjectPatch } from "@/bindings"

/** The sections of a project an edit changed. Mirrors `Touched` in Rust. */
export type Touched = {
  settings: boolean
  samples: boolean
  channels: boolean
  mixer: boolean
  playlist: boolean
  /** A pattern was added, removed or reordered. */
  patternList: boolean
  /** Patterns whose data changed, including ones added or removed. */
  patterns: PatternId[]
}

export function emptyTouched(): Touched {
  return {
    settings: false,
    samples: false,
    channels: false,
    mixer: false,
    playlist: false,
    patternList: false,
    patterns: [],
  }
}

export function touchPattern(touched: Touched, id: PatternId) {
  if (!touched.patterns.includes(id)) touched.patterns.push(id)
}

export function mergeTouched(into: Touched, other: Touched): Touched {
  const merged: Touched = {
    settings: into.settings || other.settings,
    samples: into.samples || other.samples,
    channels: into.channels || other.channels,
    mixer: into.mixer || other.mixer,
    playlist: into.playlist || other.playlist,
    patternList: into.patternList || other.patternList,
    patterns: [...into.patterns],
  }
  for (const id of other.patterns) touchPattern(merged, id)
  return merged
}

export function buildPatch(
  project: Project,
  touched: Touched,
  revision: number,
  history: HistoryView,
  dirty: boolean
): ProjectPatch {
  const patch: ProjectPatch = { revision, history, dirty }
  if (touched.settings) patch.settings = project.settings
  if (touched.samples) patch.samples = project.samples
  if (touched.channels) patch.channels = project.channels
  if (touched.mixer) patch.mixer = project.mixer
  if (touched.playlist) patch.playlist = project.playlist
  if (touched.patternList) {
    patch.patternOrder = project.patterns.map((pattern) => pattern.id)
  }
  const changed = project.patterns.filter((pattern) =>
    touched.patterns.includes(pattern.id)
  )
  if (changed.length > 0) patch.patterns = changed
  return patch
}
