import { SCALES, type ScaleId } from "./scales"

export function nextScale(
  id: unknown,
  direction: "previous" | "next"
): ScaleId | null {
  const index = SCALES.findIndex((scale) => scale.id === id)
  if (index === -1) return null
  return SCALES[index + (direction === "previous" ? -1 : 1)]?.id ?? null
}
