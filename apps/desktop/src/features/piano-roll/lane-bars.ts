/**
 * Bars of the value lane merged by pixel column. Zoomed far out, thousands
 * of notes share a few hundred columns; drawing one bar per column from the
 * lowest to the highest value in it looks the same and costs the width of
 * the lane instead of the number of notes.
 */
export class BarColumns {
  private top = new Int32Array(0)
  private bottom = new Int32Array(0)
  private used = new Uint8Array(0)
  private columns: number[] = []
  private width = 0

  /** Starts a frame. Every bar grows from `base`. */
  reset(width: number): void {
    if (this.used.length < width) {
      this.top = new Int32Array(width)
      this.bottom = new Int32Array(width)
      this.used = new Uint8Array(width)
    } else {
      for (const column of this.columns) this.used[column] = 0
    }
    this.columns = []
    this.width = width
  }

  /** Adds a bar that reaches from `base` to `y` in column `x`. */
  add(x: number, y: number, base: number): void {
    if (x < 0 || x >= this.width) return
    if (this.used[x] === 0) {
      this.used[x] = 1
      this.columns.push(x)
      this.top[x] = Math.min(y, base)
      this.bottom[x] = Math.max(y, base)
      return
    }
    if (y < this.top[x]) this.top[x] = y
    if (y > this.bottom[x]) this.bottom[x] = y
  }

  get count(): number {
    return this.columns.length
  }

  /** Calls `draw` once per column that holds a bar. */
  forEach(draw: (x: number, top: number, bottom: number) => void): void {
    for (const x of this.columns) draw(x, this.top[x], this.bottom[x])
  }
}
