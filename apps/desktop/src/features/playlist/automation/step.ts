import type { AutomationPoint } from "@/bindings"
import { MAX_AUTOMATION_POINTS } from "@/lib/units"

export type StepSample = { tick: number; value: number }

/**
 * Fill every grid tick between pointer samples, including fast or backwards
 * strokes. Samples are snapped and clamped to the clip window by the session.
 * Existing ticks update their last point (the visible side of a jump).
 * Null refuses the entire stroke when it would exceed the curve's capacity.
 */
export function writeSteps(
  points: readonly AutomationPoint[],
  from: StepSample,
  to: StepSample,
  spacing: number,
  origin: number
): AutomationPoint[] | null {
  const next = [...points]
  const indices = new Map(next.map((point, index) => [point.tick, index]))
  const write = (tick: number): boolean => {
    const share =
      from.tick === to.tick ? 1 : (tick - from.tick) / (to.tick - from.tick)
    const value = from.value + (to.value - from.value) * share
    const index = indices.get(tick)
    if (index === undefined) {
      if (next.length >= MAX_AUTOMATION_POINTS) return false
      indices.set(tick, next.length)
      next.push({ tick, value, curve: 0, hold: true })
    } else {
      next[index] = { ...next[index], value, curve: 0, hold: true }
    }
    return true
  }
  if (!write(from.tick)) return null
  const direction = Math.sign(to.tick - from.tick)
  if (direction !== 0) {
    const cell = (from.tick - origin) / spacing
    let tick =
      origin +
      (direction > 0 ? Math.floor(cell) + 1 : Math.ceil(cell) - 1) * spacing
    for (; direction * (to.tick - tick) > 0; tick += direction * spacing) {
      if (!write(tick)) return null
    }
    if (!write(to.tick)) return null
  }
  next.sort((a, b) => a.tick - b.tick)
  // Hold belongs to the outgoing segment; the whole curve's tail has none.
  const last = next.length - 1
  next[last] = { ...next[last], hold: false }
  return next
}
