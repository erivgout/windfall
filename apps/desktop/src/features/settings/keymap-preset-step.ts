import { KEYMAP_PRESETS } from "@/lib/actions/keymap"

export function nextKeymapPreset(
  id: string,
  direction: "previous" | "next"
): string | null {
  const index = KEYMAP_PRESETS.findIndex((item) => item.id === id)
  if (index === -1) return null

  const step = direction === "previous" ? -1 : 1
  return KEYMAP_PRESETS[index + step]?.id ?? null
}
