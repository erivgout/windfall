export const RECORDING_SOURCES = ["input", "postEffects", "postFader"] as const

export function nextRecordingSource(
  mode: string,
  direction: "previous" | "next"
): "input" | "postEffects" | "postFader" | null {
  const index = RECORDING_SOURCES.findIndex((item) => item === mode)
  if (index === -1) return null

  return RECORDING_SOURCES[index + (direction === "previous" ? -1 : 1)] ?? null
}
