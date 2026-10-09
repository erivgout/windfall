export const STRETCH_QUALITIES = ["fast", "standard", "high"] as const

export function nextClipStretchQuality(
  quality: string,
  direction: "previous" | "next"
): "fast" | "standard" | "high" | null {
  const index = STRETCH_QUALITIES.findIndex((item) => item === quality)
  if (index === -1) return null

  return STRETCH_QUALITIES[index + (direction === "previous" ? -1 : 1)] ?? null
}
