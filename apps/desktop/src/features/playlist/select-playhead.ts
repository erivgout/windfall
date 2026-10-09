import type { Clip, ClipId } from "@/bindings"

/** Returns clips covering the tick in their original order, excluding their end. */
export function clipsAtTick(clips: readonly Clip[], tick: number): ClipId[] {
  return clips
    .filter((clip) => clip.start <= tick && tick < clip.start + clip.length)
    .map((clip) => clip.id)
}
