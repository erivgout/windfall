import type { NoteId } from "@/bindings"

export const VELOCITY_PRESETS = [
  { label: "Soft", velocity: 0.25 },
  { label: "Medium", velocity: 0.5 },
  { label: "Strong", velocity: 0.8 },
  { label: "Full", velocity: 1 },
] as const

type VelocityNote = {
  id: NoteId
  velocity: number
}

export function velocityPresetUpdates(
  notes: readonly VelocityNote[],
  preset: number
): VelocityNote[] {
  return notes.flatMap(({ id, velocity }) =>
    Math.abs(velocity - preset) < 0.001 ? [] : [{ id, velocity: preset }]
  )
}
