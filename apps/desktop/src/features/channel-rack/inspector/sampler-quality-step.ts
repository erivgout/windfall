export const PREPARATION_QUALITIES = ["fast", "standard", "high"] as const

export function nextSamplerQuality(
  quality: string,
  direction: "previous" | "next"
): "fast" | "standard" | "high" | null {
  const index = PREPARATION_QUALITIES.findIndex((item) => item === quality)
  if (index === -1) return null

  return PREPARATION_QUALITIES[index + (direction === "previous" ? -1 : 1)] ?? null
}
