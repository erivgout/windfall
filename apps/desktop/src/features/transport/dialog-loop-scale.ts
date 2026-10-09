import { MAX_SONG_TICKS } from "@/lib/units"

export function scaledDialogLoopEnd(start: number, end: number, factor: "half" | "double"): number {
  const span = Math.max(1, end - start)
  if (factor === "half") return start + Math.max(1, Math.floor(span / 2))
  return Math.max(start + 1, Math.min(MAX_SONG_TICKS, start + span * 2))
}

export function nextDialogLoopScale(start: number, end: number, factor: "half" | "double"): number | null {
  const next = scaledDialogLoopEnd(start, end, factor)
  return next === end ? null : next
}
