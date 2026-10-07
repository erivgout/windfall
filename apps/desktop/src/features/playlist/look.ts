import type {
  Automation,
  AutomationId,
  Clip,
  ClipContent,
  MixerTrack,
  Pattern,
  PatternId,
  Project,
  SampleAsset,
  SampleId,
  TrackId,
} from "@/bindings"

/** Shown for a clip whose content cannot be found. It should never be seen. */
export const ORPHAN_COLOR = 0x8b8d98

/**
 * What the clips of a project play, by id. A clip names its content by id
 * only, so drawing one means looking its pattern, sample, mixer track or
 * automation up here.
 */
export type ClipLookups = {
  readonly patterns: ReadonlyMap<PatternId, Pattern>
  readonly samples: ReadonlyMap<SampleId, SampleAsset>
  readonly mixerTracks: ReadonlyMap<TrackId, MixerTrack>
  readonly automations: ReadonlyMap<AutomationId, Automation>
}

export const NO_LOOKUPS: ClipLookups = {
  patterns: new Map(),
  samples: new Map(),
  mixerTracks: new Map(),
  automations: new Map(),
}

export function lookupsOf(
  project: Pick<Project, "patterns" | "samples" | "mixer" | "automations">
): ClipLookups {
  return {
    patterns: new Map(project.patterns.map((item) => [item.id, item])),
    samples: new Map(project.samples.map((item) => [item.id, item])),
    mixerTracks: new Map(project.mixer.tracks.map((item) => [item.id, item])),
    automations: new Map(project.automations.map((item) => [item.id, item])),
  }
}

/**
 * The color of a clip: its pattern's, its automation's, or for audio the
 * color of the mixer track it plays into.
 */
export function contentColor(
  content: ClipContent,
  lookups: ClipLookups
): number {
  switch (content.type) {
    case "pattern":
      return lookups.patterns.get(content.pattern)?.color ?? ORPHAN_COLOR
    case "audio":
      return lookups.mixerTracks.get(content.mixerTrack)?.color ?? ORPHAN_COLOR
    case "automation":
      return lookups.automations.get(content.automation)?.color ?? ORPHAN_COLOR
    default: {
      const _exhaustive: never = content
      return _exhaustive
    }
  }
}

/** The name drawn on a clip. Empty when what it plays is gone. */
export function contentName(
  content: ClipContent,
  lookups: ClipLookups
): string {
  switch (content.type) {
    case "pattern":
      return lookups.patterns.get(content.pattern)?.name ?? ""
    case "audio":
      return lookups.samples.get(content.sample)?.name ?? ""
    case "automation":
      return lookups.automations.get(content.automation)?.name ?? ""
    default: {
      const _exhaustive: never = content
      return _exhaustive
    }
  }
}

/** The pattern a clip plays, for the kinds of clip that play one. */
export function clipPattern(
  clip: Pick<Clip, "content">,
  lookups: ClipLookups
): Pattern | undefined {
  return clip.content.type === "pattern"
    ? lookups.patterns.get(clip.content.pattern)
    : undefined
}

/**
 * What the batch on the canvas depends on, as text: the colors of
 * everything a clip can take its color from. A fader move changes a mixer
 * track but not this, so the batch is not rebuilt for it.
 */
export function colorSignature(lookups: ClipLookups): string {
  const parts: number[] = []
  for (const item of lookups.patterns.values()) parts.push(item.id, item.color)
  for (const item of lookups.mixerTracks.values()) {
    parts.push(item.id, item.color)
  }
  for (const item of lookups.automations.values()) {
    parts.push(item.id, item.color)
  }
  return parts.join(",")
}
