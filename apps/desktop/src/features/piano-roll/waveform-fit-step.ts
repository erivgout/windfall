export const WAVEFORM_FITS = ["seconds", "pattern", "custom"] as const

export function nextWaveformFit(
  fit: string,
  direction: "previous" | "next"
): "seconds" | "pattern" | "custom" | null {
  const index = WAVEFORM_FITS.findIndex((item) => item === fit)
  if (index === -1) return null

  return WAVEFORM_FITS[index + (direction === "previous" ? -1 : 1)] ?? null
}
