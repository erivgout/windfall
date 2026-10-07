export type Selection = { start: number; end: number }

export function frameAt(
  x: number,
  left: number,
  width: number,
  frames: number
) {
  if (width <= 0 || !Number.isFinite(x)) return 0
  return Math.round(Math.max(0, Math.min(1, (x - left) / width)) * frames)
}

export function selectionBetween(
  a: number,
  b: number,
  frames: number
): Selection {
  const start = Math.max(0, Math.min(frames - 1, Math.min(a, b)))
  return { start, end: Math.max(start + 1, Math.min(frames, Math.max(a, b))) }
}

export function validSelection(selection: Selection, frames: number) {
  return (
    Number.isInteger(selection.start) &&
    Number.isInteger(selection.end) &&
    selection.start >= 0 &&
    selection.start < selection.end &&
    selection.end <= frames
  )
}
