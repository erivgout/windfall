import { SNAP_MODES, type SnapMode } from "./snap"

const grids = SNAP_MODES.filter((item) => item.mode !== "none").reverse()

export function nextPlaylistSnapScale(
  mode: string,
  direction: "finer" | "coarser"
): SnapMode | null {
  const index = grids.findIndex((item) => item.mode === mode)
  if (index === -1) return null

  return grids[index + (direction === "finer" ? -1 : 1)]?.mode ?? null
}
