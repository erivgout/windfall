import { PPQ } from "@/lib/units"

export const SLICE_GRIDS = [PPQ / 4, PPQ / 2, PPQ, PPQ * 2, PPQ * 4]

export function nextSliceGrid(
  ticks: number,
  direction: "finer" | "coarser"
): number | null {
  const index = SLICE_GRIDS.indexOf(ticks)
  if (index === -1) return null
  return SLICE_GRIDS[index + (direction === "finer" ? -1 : 1)] ?? null
}
