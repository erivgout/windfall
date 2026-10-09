import type { HistoryRun } from "./history-list"

/** Keep matching runs in order, with their original undo indexes. */
export function matchingRuns(runs: HistoryRun[], query: string): HistoryRun[] {
  const filter = query.trim().toLowerCase()
  if (!filter) return runs
  return runs.filter((run) => run.label.toLowerCase().includes(filter))
}
