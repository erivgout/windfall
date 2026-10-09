import { ROOT_KEY_PRESETS } from "./root-key-presets"

export function nextRootKeyPreset(
  rootKey: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(rootKey)) return null

  const index = ROOT_KEY_PRESETS.findIndex((item) => item.rootKey === rootKey)
  if (index !== -1) {
    const nextIndex = index + (direction === "previous" ? -1 : 1)
    return ROOT_KEY_PRESETS[nextIndex]?.rootKey ?? null
  }

  let previous: number | null = null
  for (const item of ROOT_KEY_PRESETS) {
    if (item.rootKey > rootKey) {
      return direction === "previous" ? previous : item.rootKey
    }
    previous = item.rootKey
  }
  return direction === "previous" ? previous : null
}
