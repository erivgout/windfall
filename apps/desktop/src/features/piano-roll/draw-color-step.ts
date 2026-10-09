import { NOTE_COLOR_GROUPS } from "@/lib/note-colors"

export function nextDrawColor(
  current: number | null,
  direction: "previous" | "next"
): { color: number | null } | null {
  if (current === null) {
    return direction === "next" ? { color: 0 } : null
  }
  if (
    !Number.isInteger(current) ||
    current < 0 ||
    current >= NOTE_COLOR_GROUPS.length
  ) {
    return null
  }

  if (direction === "previous") {
    return { color: current === 0 ? null : current - 1 }
  }
  return current === NOTE_COLOR_GROUPS.length - 1
    ? null
    : { color: current + 1 }
}
