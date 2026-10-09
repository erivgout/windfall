import { MAX_SONG_TICKS } from "@/lib/units"

export function scaledDialogStart(start: number, factor: "half" | "double"): number {
  if (factor === "half") return Math.max(0, Math.floor(start / 2))
  return Math.min(MAX_SONG_TICKS, start * 2)
}

export function nextDialogStartScale(start: number, factor: "half" | "double"): number | null {
  const next = scaledDialogStart(start, factor)
  return next === start ? null : next
}
