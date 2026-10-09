const countInBarCounts = [0, 1, 2, 4, 8]

export function scaledCountIn(bars: number, factor: "half" | "double"): number | null {
  const index = countInBarCounts.indexOf(bars)
  if (index === -1) return null
  if (bars === 0) return 0
  const nextIndex = factor === "half" ? index - 1 : Math.min(index + 1, countInBarCounts.length - 1)
  return countInBarCounts[nextIndex]
}

export function nextCountInScale(bars: number, factor: "half" | "double"): number | null {
  const next = scaledCountIn(bars, factor)
  return next === bars ? null : next
}
