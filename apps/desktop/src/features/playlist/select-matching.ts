import type { Clip, ClipContent, ClipId } from "@/bindings"

function sourceKey(content: ClipContent): string {
  switch (content.type) {
    case "pattern":
      return `pattern:${content.pattern}`
    case "audio":
      return `audio:${content.sample}`
    case "automation":
      return `automation:${content.automation}`
  }
}

/** Keeps selected clips first, then adds clips sharing their kind and source. */
export function selectMatchingClips(
  selected: readonly Clip[],
  clips: readonly Clip[]
): ClipId[] {
  const ids = new Set(selected.map((clip) => clip.id))
  if (ids.size === 0) return []

  const sources = new Set(selected.map((clip) => sourceKey(clip.content)))
  for (const clip of clips) {
    if (sources.has(sourceKey(clip.content))) ids.add(clip.id)
  }
  return [...ids]
}
