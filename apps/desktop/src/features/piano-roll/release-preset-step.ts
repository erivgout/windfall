import { RELEASE_PRESETS } from "./release-presets"

export function nextNoteReleasePreset(
  release: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(release)) return null

  const index = RELEASE_PRESETS.findIndex(
    (preset) => Math.abs(release - preset.release) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return RELEASE_PRESETS[index + step]?.release ?? null
  }

  let previous: number | null = null
  for (const preset of RELEASE_PRESETS) {
    if (preset.release > release) {
      return direction === "previous" ? previous : preset.release
    }
    previous = preset.release
  }
  return direction === "previous" ? previous : null
}
