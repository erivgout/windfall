import type { Clip, ClipId } from "@/bindings"

/** Keeps pattern and audio clip ids in selection order, skipping automation. */
export function makeUniqueClipIds(selected: readonly Clip[]): ClipId[] {
  return selected
    .filter(
      (clip) => clip.content.type === "pattern" || clip.content.type === "audio"
    )
    .map((clip) => clip.id)
}
