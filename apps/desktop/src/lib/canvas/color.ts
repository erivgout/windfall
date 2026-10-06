/** Straight (not premultiplied) color. Every channel is 0 to 255. */
export interface Rgba {
  readonly r: number
  readonly g: number
  readonly b: number
  readonly a: number
}

export function rgba(r: number, g: number, b: number, a = 255): Rgba {
  return { r, g, b, a }
}

/** `0xRRGGBB`, the project model's color format, to an opaque color. */
export function rgbFromInt(color: number): Rgba {
  return {
    r: (color >> 16) & 0xff,
    g: (color >> 8) & 0xff,
    b: color & 0xff,
    a: 255,
  }
}

export function withAlpha(color: Rgba, factor: number): Rgba {
  return { ...color, a: Math.round(color.a * factor) }
}

/** Linear mix in sRGB bytes. `t` 0 gives `from`, 1 gives `to`. */
export function mix(from: Rgba, to: Rgba, t: number): Rgba {
  return {
    r: Math.round(from.r + (to.r - from.r) * t),
    g: Math.round(from.g + (to.g - from.g) * t),
    b: Math.round(from.b + (to.b - from.b) * t),
    a: Math.round(from.a + (to.a - from.a) * t),
  }
}

export function rgbaToCss(color: Rgba): string {
  return color.a >= 255
    ? `rgb(${color.r},${color.g},${color.b})`
    : `rgba(${color.r},${color.g},${color.b},${(color.a / 255).toFixed(4)})`
}

/** One number per color, for map keys and change detection. */
export function packRgba(color: Rgba): number {
  return ((color.r << 24) | (color.g << 16) | (color.b << 8) | color.a) >>> 0
}

export function unpackRgba(packed: number): Rgba {
  return {
    r: (packed >>> 24) & 0xff,
    g: (packed >>> 16) & 0xff,
    b: (packed >>> 8) & 0xff,
    a: packed & 0xff,
  }
}
