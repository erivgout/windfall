import { clamp, DEFAULT_KEY } from "@/lib/units"

/** Scales the whole-key distance from C5. */
export function scaledRootKey(
  key: number,
  factor: "half" | "double"
): number {
  const distance = key - DEFAULT_KEY
  return factor === "half"
    ? DEFAULT_KEY + Math.trunc(distance / 2)
    : clamp(DEFAULT_KEY + distance * 2, 0, 127)
}

export function nextRootKeyScale(
  key: number,
  factor: "half" | "double"
): number | null {
  const next = scaledRootKey(key, factor)
  return next === key ? null : next
}
