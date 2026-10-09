export const ROOT_KEY_PRESETS = [
  { label: "C3", rootKey: 36 },
  { label: "C4", rootKey: 48 },
  { label: "C5", rootKey: 60 },
  { label: "C6", rootKey: 72 },
] as const

export function nextRootKey(current: number, preset: number): number | null {
  return current === preset ? null : preset
}
