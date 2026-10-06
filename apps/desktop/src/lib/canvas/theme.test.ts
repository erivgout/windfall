import { describe, expect, it } from "vitest"

import { mix, packRgba, rgba, rgbFromInt, rgbaToCss, unpackRgba } from "./color"
import { buildNoteBatch, velocityPalette, VELOCITY_LEVELS } from "./notes"
import { RECT_SELECTED } from "./rect-batch"
import {
  deriveGridTheme,
  levelColor,
  readThemeTokens,
  type CssColorParser,
  type ThemeTokens,
} from "./theme"

const dark: ThemeTokens = {
  background: rgba(20, 20, 20),
  foreground: rgba(250, 250, 250),
  mutedForeground: rgba(160, 160, 160),
  brand: rgba(230, 60, 140),
  playhead: rgba(90, 160, 255),
  gridLine: rgba(255, 255, 255, 18),
  gridLineStrong: rgba(255, 255, 255, 51),
}

describe("color helpers", () => {
  it("converts the project model's integer colors", () => {
    expect(rgbFromInt(0xff8000)).toEqual({ r: 255, g: 128, b: 0, a: 255 })
  })

  it("packs and unpacks without loss", () => {
    const color = rgba(255, 128, 3, 200)
    expect(unpackRgba(packRgba(color))).toEqual(color)
    expect(packRgba(rgba(255, 255, 255, 255))).toBe(0xffffffff)
  })

  it("writes CSS colors", () => {
    expect(rgbaToCss(rgba(1, 2, 3))).toBe("rgb(1,2,3)")
    expect(rgbaToCss(rgba(1, 2, 3, 51))).toBe("rgba(1,2,3,0.2000)")
  })

  it("mixes", () => {
    expect(mix(rgba(0, 0, 0), rgba(200, 100, 50), 0.5)).toEqual(
      rgba(100, 50, 25)
    )
  })
})

describe("readThemeTokens", () => {
  const parse: CssColorParser = (css) => {
    const match = /^rgb\((\d+) (\d+) (\d+)\)$/.exec(css)
    return match
      ? rgba(Number(match[1]), Number(match[2]), Number(match[3]))
      : null
  }

  it("reads the CSS variables from the element", () => {
    const element = document.createElement("div")
    element.style.setProperty("--background", "rgb(1 2 3)")
    element.style.setProperty("--wf-playhead", " rgb(4 5 6) ")
    document.body.append(element)
    const tokens = readThemeTokens(element, parse)
    element.remove()
    expect(tokens.background).toEqual(rgba(1, 2, 3))
    expect(tokens.playhead).toEqual(rgba(4, 5, 6))
  })

  it("falls back when a variable is missing or unparseable", () => {
    const element = document.createElement("div")
    element.style.setProperty("--wf-brand", "not a color")
    document.body.append(element)
    const tokens = readThemeTokens(element, parse)
    element.remove()
    expect(tokens.brand.a).toBe(255)
    expect(tokens.gridLine.a).toBeGreaterThan(0)
  })
})

describe("deriveGridTheme", () => {
  const theme = deriveGridTheme(dark)

  it("keeps grid colors translucent so they work over row shading", () => {
    expect(theme.gridBar).toEqual(dark.gridLineStrong)
    expect(theme.gridMinor.a).toBeLessThan(theme.gridBeat.a)
    expect(theme.rowShade.a).toBeLessThan(40)
  })

  it("takes item, playhead and selection colors from the tokens", () => {
    expect(theme.item).toEqual(dark.brand)
    expect(theme.playhead).toEqual(dark.playhead)
    expect(theme.selectionBorder).toEqual(dark.foreground)
  })
})

describe("note colors", () => {
  const theme = deriveGridTheme(dark)

  it("fades quiet notes toward the background in either theme", () => {
    const loud = levelColor(theme, theme.item, 1)
    const quiet = levelColor(theme, theme.item, 0)
    expect(loud).toEqual(theme.item)
    expect(quiet.r).toBeLessThan(loud.r)
    expect(quiet.r).toBeGreaterThan(theme.background.r)

    const light = deriveGridTheme({ ...dark, background: rgba(255, 255, 255) })
    const quietOnLight = levelColor(light, light.item, 0)
    expect(quietOnLight.g).toBeGreaterThan(light.item.g)
    expect(quietOnLight.a).toBe(255)
  })

  it("builds one palette entry per velocity level", () => {
    const palette = velocityPalette(theme)
    expect(palette).toHaveLength(VELOCITY_LEVELS)
    expect(palette[VELOCITY_LEVELS - 1]).toEqual(theme.item)
  })

  it("turns notes into rects with the highest key on the top row", () => {
    const palette = velocityPalette(theme)
    const batch = buildNoteBatch(
      [
        { id: 5, start: 960, length: 240, key: 60, velocity: 1 },
        { id: 6, start: 0, length: 480, key: 127, velocity: 0 },
        { id: 7, start: 240, length: 120, key: 0, velocity: 2 },
      ],
      palette,
      { selected: new Set([6]) }
    )
    expect(batch.count).toBe(3)
    expect([
      batch.start(0),
      batch.length(0),
      batch.row(0),
      batch.rowSpan(0),
    ]).toEqual([960, 240, 67, 1])
    expect(batch.row(1)).toBe(0)
    expect(batch.row(2)).toBe(127)
    expect(batch.flags[1]).toBe(RECT_SELECTED)
    expect(batch.selectedCount).toBe(1)
    expect([...batch.colors.subarray(0, 4)]).toEqual([230, 60, 140, 255])
    expect(batch.colors[4]).toBe(palette[0].r)
    // Velocity above 1 clamps to the loudest color.
    expect(batch.colors[8]).toBe(230)
  })
})
