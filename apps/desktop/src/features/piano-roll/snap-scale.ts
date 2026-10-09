import { SNAP_OPTIONS, type SnapId } from "./snap"

export function nextSnapScale(
  id: string,
  direction: "finer" | "coarser"
): SnapId | null {
  const options = SNAP_OPTIONS.filter((option) => option.id !== "none")
  const index = options.findIndex((option) => option.id === id)
  if (index === -1) return null

  return options[index + (direction === "finer" ? -1 : 1)]?.id ?? null
}
