import { MAX_LANE_HEIGHT, MIN_LANE_HEIGHT } from "./store"

export function scaledLaneHeight(
  height: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.max(MIN_LANE_HEIGHT, Math.floor(height / 2))
    : Math.min(MAX_LANE_HEIGHT, height * 2)
}

export function nextLaneHeightScale(
  height: number,
  factor: "half" | "double"
): number | null {
  const next = scaledLaneHeight(height, factor)
  return next === height ? null : next
}
