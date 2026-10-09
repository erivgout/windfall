import type { NoteId } from "@/bindings"

type VelocityScaleFactor = "half" | "double"

type VelocityNote = {
  id: NoteId
  velocity: number
}

export function scaledVelocity(
  velocity: number,
  factor: VelocityScaleFactor
): number {
  return factor === "half" ? velocity / 2 : Math.min(velocity * 2, 1)
}

export function velocityScaleUpdates(
  notes: readonly VelocityNote[],
  factor: VelocityScaleFactor
): VelocityNote[] {
  return notes.flatMap(({ id, velocity }) => {
    const next = scaledVelocity(velocity, factor)
    return Math.abs(next - velocity) < 0.001 ? [] : [{ id, velocity: next }]
  })
}
