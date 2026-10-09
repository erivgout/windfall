import { NOTE_PAN_PRESETS } from "./note-pan-presets"

export function nextNotePanPreset(
  pan: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(pan)) return null

  const index = NOTE_PAN_PRESETS.findIndex(
    (preset) => Math.abs(pan - preset.pan) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return NOTE_PAN_PRESETS[index + step]?.pan ?? null
  }

  let previous: number | null = null
  for (const preset of NOTE_PAN_PRESETS) {
    if (preset.pan > pan) {
      return direction === "previous" ? previous : preset.pan
    }
    previous = preset.pan
  }
  return direction === "previous" ? previous : null
}
