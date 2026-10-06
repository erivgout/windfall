import { mix, rgba, withAlpha, type Rgba } from "./color"

/** The CSS variables canvas drawing depends on, resolved to bytes. */
export interface ThemeTokens {
  readonly background: Rgba
  readonly foreground: Rgba
  readonly mutedForeground: Rgba
  readonly brand: Rgba
  readonly playhead: Rgba
  readonly gridLine: Rgba
  readonly gridLineStrong: Rgba
}

/** Every color a time-grid editor draws with. */
export interface GridTheme {
  readonly background: Rgba
  readonly foreground: Rgba
  readonly mutedForeground: Rgba
  /** Shading that sets marked rows (black keys in the piano roll) apart. */
  readonly rowShade: Rgba
  /**
   * True when `rowShade` is lighter than the background, as in the dark
   * theme. The shade then goes on the unmarked rows, so the marked rows
   * are the darker ones in both themes.
   */
  readonly rowShadeOnUnmarked: boolean
  readonly rowLine: Rgba
  /** Row lines between groups (octaves in the piano roll). */
  readonly rowLineStrong: Rgba
  readonly gridMinor: Rgba
  readonly gridBeat: Rgba
  readonly gridBar: Rgba
  /** Default item color at full brightness. */
  readonly item: Rgba
  /** Selected items are mixed toward this color by `selectionMix`. */
  readonly selectionFill: Rgba
  readonly selectionMix: number
  readonly selectionBorder: Rgba
  readonly marqueeFill: Rgba
  readonly marqueeStroke: Rgba
  readonly playhead: Rgba
}

export type CssColorParser = (css: string) => Rgba | null

const TOKEN_VARIABLES: Record<keyof ThemeTokens, string> = {
  background: "--background",
  foreground: "--foreground",
  mutedForeground: "--muted-foreground",
  brand: "--wf-brand",
  playhead: "--wf-playhead",
  gridLine: "--wf-grid-line",
  gridLineStrong: "--wf-grid-line-strong",
}

// Used only when a variable is missing, so a page without the stylesheet
// still draws something readable.
const FALLBACK_TOKENS: ThemeTokens = {
  background: rgba(255, 255, 255),
  foreground: rgba(20, 20, 20),
  mutedForeground: rgba(120, 120, 120),
  brand: rgba(214, 51, 132),
  playhead: rgba(40, 110, 220),
  gridLine: rgba(0, 0, 0, 20),
  gridLineStrong: rgba(0, 0, 0, 56),
}

/**
 * Parses any CSS color the browser understands, including `oklch()`, by
 * painting one pixel and reading it back. Colors outside sRGB are clamped.
 */
export function createCanvasColorParser(): CssColorParser {
  const canvas = document.createElement("canvas")
  canvas.width = 1
  canvas.height = 1
  const ctx = canvas.getContext("2d", { willReadFrequently: true })
  return (css) => {
    if (!ctx || css === "") return null
    // An invalid color leaves fillStyle unchanged, so two different
    // starting values expose it.
    ctx.fillStyle = "#000000"
    ctx.fillStyle = css
    const first = ctx.fillStyle
    ctx.fillStyle = "#ffffff"
    ctx.fillStyle = css
    if (ctx.fillStyle !== first) return null
    ctx.clearRect(0, 0, 1, 1)
    ctx.fillRect(0, 0, 1, 1)
    const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data
    return { r, g, b, a }
  }
}

export function readThemeTokens(
  element: Element,
  parse: CssColorParser
): ThemeTokens {
  const style = getComputedStyle(element)
  const read = (key: keyof ThemeTokens): Rgba =>
    parse(style.getPropertyValue(TOKEN_VARIABLES[key]).trim()) ??
    FALLBACK_TOKENS[key]
  return {
    background: read("background"),
    foreground: read("foreground"),
    mutedForeground: read("mutedForeground"),
    brand: read("brand"),
    playhead: read("playhead"),
    gridLine: read("gridLine"),
    gridLineStrong: read("gridLineStrong"),
  }
}

export function deriveGridTheme(tokens: ThemeTokens): GridTheme {
  return {
    background: tokens.background,
    foreground: tokens.foreground,
    mutedForeground: tokens.mutedForeground,
    rowShade: withAlpha(tokens.gridLine, 0.75),
    rowShadeOnUnmarked:
      luminance(tokens.gridLine) > luminance(tokens.background),
    rowLine: withAlpha(tokens.gridLine, 0.6),
    rowLineStrong: withAlpha(tokens.gridLineStrong, 0.6),
    gridMinor: withAlpha(tokens.gridLine, 0.6),
    gridBeat: tokens.gridLine,
    gridBar: tokens.gridLineStrong,
    item: tokens.brand,
    selectionFill: tokens.foreground,
    selectionMix: 0.4,
    selectionBorder: tokens.foreground,
    marqueeFill: withAlpha(tokens.playhead, 0.16),
    marqueeStroke: tokens.playhead,
    playhead: tokens.playhead,
  }
}

function luminance(color: Rgba): number {
  return 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

export function readGridTheme(
  element: Element,
  parse: CssColorParser
): GridTheme {
  return deriveGridTheme(readThemeTokens(element, parse))
}

/**
 * Fill color for an item at a brightness level from 0 to 1 (note velocity).
 * A quiet note fades toward the background, which reads correctly in both
 * the light and the dark theme.
 */
export function levelColor(theme: GridTheme, base: Rgba, level: number): Rgba {
  const t = Math.min(1, Math.max(0, level))
  return mix(theme.background, base, 0.38 + 0.62 * t)
}

/**
 * Calls `onChange` when the theme may have changed: a class, style or
 * `data-theme` change on the root element or body, or the system scheme.
 */
export function observeTheme(onChange: () => void): () => void {
  const observer = new MutationObserver(onChange)
  const attributes = {
    attributes: true,
    attributeFilter: ["class", "style", "data-theme"],
  }
  observer.observe(document.documentElement, attributes)
  if (document.body) observer.observe(document.body, attributes)
  const media = window.matchMedia("(prefers-color-scheme: dark)")
  media.addEventListener("change", onChange)
  return () => {
    observer.disconnect()
    media.removeEventListener("change", onChange)
  }
}
