export type LightSource = "hover" | "selected" | "playing" | "dragged"

/**
 * Which keys of the gutter keyboard are lit, and why. A key stays lit
 * while any source holds it, and the keyboard is only told about keys that
 * actually turn on or off.
 */
export class KeyLights {
  private apply: ((key: number, lit: boolean) => void) | null = null
  private sources = new Map<LightSource, ReadonlySet<number>>()
  private lit = new Set<number>()

  /** Connects the keyboard to light, or null when it goes away. */
  attach(apply: ((key: number, lit: boolean) => void) | null): void {
    this.apply = apply
    if (apply) for (const key of this.lit) apply(key, true)
  }

  set(source: LightSource, keys: Iterable<number>): void {
    this.sources.set(source, new Set(keys))
    this.refresh()
  }

  private refresh(): void {
    const next = new Set<number>()
    for (const keys of this.sources.values()) {
      for (const key of keys) next.add(key)
    }
    for (const key of this.lit) {
      if (!next.has(key)) this.apply?.(key, false)
    }
    for (const key of next) {
      if (!this.lit.has(key)) this.apply?.(key, true)
    }
    this.lit = next
  }
}
