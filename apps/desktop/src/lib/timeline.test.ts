import { describe, expect, it } from "vitest"
import fixtures from "../../../../crates/windfall-project/tests/timeline/meter-fixtures.json"
import { sim } from "./ipc/sim/wasm"
import { formatMusicalPosition, musicalPosition, rulerLabels } from "./timeline"

describe("shared Rust meter fixtures", () => {
  it("matches actual WASM conversions and ruler labels at shortened bars", () => {
    for (const entry of fixtures.positions) {
      const input = {
        legacy: fixtures.legacy,
        meters: fixtures.meters,
        tick: entry.tick,
      }
      expect(sim.call("timeline_position", 0, input)).toEqual(entry.position)
      expect(
        musicalPosition(entry.tick, fixtures.legacy, fixtures.meters)
      ).toEqual(entry.position)
    }
    expect(
      rulerLabels(0, 10280, 0.1, fixtures.legacy, fixtures.meters)
    ).toEqual([
      { tick: 0, bar: 1 },
      { tick: 3840, bar: 2 },
      { tick: 4001, bar: 3 },
      { tick: 7361, bar: 4 },
      { tick: 7400, bar: 5 },
      { tick: 10280, bar: 6 },
    ])
    expect(() => sim.call("timeline_range", 0, { start: 4, end: 4 })).toThrow(
      /range/
    )
    expect(() =>
      sim.call("timeline_position", 0, {
        legacy: { numerator: 4, denominator: 0 },
        meters: [],
        tick: 3,
      })
    ).toThrow()
    expect(formatMusicalPosition(4001, fixtures.legacy, fixtures.meters)).toBe(
      "003:01:1"
    )
    expect(formatMusicalPosition(7400, fixtures.legacy, fixtures.meters)).toBe(
      "005:01:1"
    )
  })
})
