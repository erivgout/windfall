import { DEFAULT_ROW_HEIGHT, MIN_ROW_HEIGHT, TALL_ROW_HEIGHT } from "./layout"

export const TRACK_HEIGHT_PRESETS = [
  { label: "Follow", height: 0 },
  { label: "Short", height: MIN_ROW_HEIGHT },
  { label: "Normal", height: DEFAULT_ROW_HEIGHT },
  { label: "Tall", height: TALL_ROW_HEIGHT },
] as const

export function nextTrackHeight(
  current: number,
  preset: number
): number | null {
  return current === preset ? null : preset
}
