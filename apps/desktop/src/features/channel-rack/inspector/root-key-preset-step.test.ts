import { describe, expect, it } from "vitest"

import { nextRootKeyPreset } from "./root-key-preset-step"
import { ROOT_KEY_PRESETS } from "./root-key-presets"

const [first, second, third, fourth] = ROOT_KEY_PRESETS

describe("root key preset step", () => {
  it("lists C3 through C6 as keys 36, 48, 60, and 72", () => {
    expect(ROOT_KEY_PRESETS.map((item) => item.rootKey)).toEqual([36, 48, 60, 72])
  })

  it.each([
    { rootKey: first.rootKey, previous: null, next: second.rootKey },
    { rootKey: second.rootKey, previous: first.rootKey, next: third.rootKey },
    { rootKey: third.rootKey, previous: second.rootKey, next: fourth.rootKey },
    { rootKey: fourth.rootKey, previous: third.rootKey, next: null },
    { rootKey: 30, previous: null, next: first.rootKey },
    { rootKey: 40, previous: first.rootKey, next: second.rootKey },
    { rootKey: 50, previous: second.rootKey, next: third.rootKey },
    { rootKey: 66, previous: third.rootKey, next: fourth.rootKey },
    { rootKey: 80, previous: fourth.rootKey, next: null },
    { rootKey: 60.5, previous: third.rootKey, next: fourth.rootKey },
    {
      rootKey: third.rootKey - 0.0004,
      previous: second.rootKey,
      next: third.rootKey,
    },
    {
      rootKey: third.rootKey + 0.0004,
      previous: third.rootKey,
      next: fourth.rootKey,
    },
  ])(
    "steps root key $rootKey to its neighboring presets",
    ({ rootKey, previous, next }) => {
      expect(nextRootKeyPreset(rootKey, "previous")).toBe(previous)
      expect(nextRootKeyPreset(rootKey, "next")).toBe(next)
    }
  )

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for non-finite root key %s",
    (rootKey) => {
      expect(nextRootKeyPreset(rootKey, "previous")).toBeNull()
      expect(nextRootKeyPreset(rootKey, "next")).toBeNull()
    }
  )
})
