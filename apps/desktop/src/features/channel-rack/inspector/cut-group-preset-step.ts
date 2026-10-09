import { CUT_GROUP_PRESETS } from "./cut-group-presets"

export function nextCutGroupPreset(
  cutGroup: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(cutGroup)) return null

  const index = CUT_GROUP_PRESETS.findIndex((item) => item.cutGroup === cutGroup)
  if (index !== -1) {
    const nextIndex = index + (direction === "previous" ? -1 : 1)
    return CUT_GROUP_PRESETS[nextIndex]?.cutGroup ?? null
  }

  let previous: number | null = null
  for (const item of CUT_GROUP_PRESETS) {
    if (item.cutGroup > cutGroup) {
      return direction === "previous" ? previous : item.cutGroup
    }
    previous = item.cutGroup
  }
  return direction === "previous" ? previous : null
}
