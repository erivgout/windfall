export interface Summary {
  readonly samples: number
  readonly avg: number
  readonly p50: number
  readonly p95: number
  readonly p99: number
  readonly max: number
}

/** Nearest-rank percentile of an ascending array. */
export function percentile(sorted: readonly number[], p: number): number {
  if (sorted.length === 0) return 0
  const rank = Math.ceil((p / 100) * sorted.length)
  return sorted[Math.min(sorted.length - 1, Math.max(0, rank - 1))]
}

export function summarize(values: readonly number[]): Summary {
  if (values.length === 0) {
    return { samples: 0, avg: 0, p50: 0, p95: 0, p99: 0, max: 0 }
  }
  const sorted = [...values].sort((a, b) => a - b)
  let total = 0
  for (const value of sorted) total += value
  return {
    samples: sorted.length,
    avg: total / sorted.length,
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    p99: percentile(sorted, 99),
    max: sorted[sorted.length - 1],
  }
}

/** Share of values above a threshold, as a percentage. */
export function percentOver(
  values: readonly number[],
  threshold: number
): number {
  if (values.length === 0) return 0
  let over = 0
  for (const value of values) if (value > threshold) over++
  return (100 * over) / values.length
}
