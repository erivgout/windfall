/** Sixteen stable note groups. Persist the group index, never its UI color. */
export const NOTE_COLOR_GROUPS = [
  0x67c98a, 0x78c7b6, 0x64bed1, 0x72a9ee,
  0x8d94ed, 0xb08ae0, 0xd586d2, 0xe488af,
  0xed8e89, 0xeaad78, 0xe2c56e, 0xc5d978,
  0x98d788, 0x83b6a3, 0xb6a9d3, 0xd6b69d,
] as const

export function isNoteColorGroup(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value < NOTE_COLOR_GROUPS.length
}

export function noteColorGroupLabel(group: number | null): string {
  return group === null ? "Channel color · MIDI auto" : `Color ${group + 1} · MIDI ${group + 1}`
}
