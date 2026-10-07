/*
 * Stand-ins for the canvas contexts jsdom does not have, shared by the
 * tests of this folder. They record what was called; nothing is drawn.
 * Not part of the app.
 */

export interface FakeGl {
  /** What `canvas.getContext("webgl2")` should return. */
  readonly context: WebGL2RenderingContext
  /** The name of every method called, in order. */
  readonly calls: string[]
  /** How often a method was called. */
  count(name: string): number
  /** Makes the context report itself lost, as the browser does. */
  lose(): void
  /** Makes it usable again. */
  restore(): void
  /** From now on no program can be made: `createProgram` returns null. */
  breakPrograms(): void
}

/** A WebGL2 context that accepts every call the renderer makes. */
export function createFakeGl(): FakeGl {
  const calls: string[] = []
  let lost = false
  let broken = false
  const lose = () => {
    lost = true
  }
  const make = () => (lost ? null : {})
  const answers: Record<string, (...args: unknown[]) => unknown> = {
    getExtension: (name) =>
      name === "WEBGL_lose_context" ? { loseContext: lose } : null,
    getParameter: () => "Fake GPU",
    getShaderParameter: () => true,
    getProgramParameter: () => true,
    getUniformLocation: () => ({}),
    isContextLost: () => lost,
    createProgram: () => (broken ? null : make()),
    createShader: make,
    createVertexArray: make,
    createBuffer: make,
    createQuery: make,
  }
  const context = new Proxy(
    {},
    {
      // Constants are read and never called, so any value does for them.
      get: (_, name) => {
        const method = String(name)
        return (...args: unknown[]) => {
          calls.push(method)
          return answers[method]?.(...args)
        }
      },
    }
  )
  return {
    context: context as WebGL2RenderingContext,
    calls,
    count: (name) => calls.filter((call) => call === name).length,
    lose,
    restore() {
      lost = false
    },
    breakPrograms() {
      broken = true
    },
  }
}

export interface Fill {
  readonly style: string
  readonly rect: readonly [number, number, number, number]
}

export interface Fake2D {
  readonly context: CanvasRenderingContext2D
  /** Every `fillRect`, with the style it was filled with. */
  readonly fills: Fill[]
}

/** A 2D context that records its fills. */
export function createFake2D(): Fake2D {
  const fills: Fill[] = []
  const context = {
    fillStyle: "",
    setTransform() {},
    clearRect() {},
    save() {},
    restore() {},
    fillRect(x: number, y: number, w: number, h: number) {
      fills.push({ style: this.fillStyle, rect: [x, y, w, h] })
    },
  }
  return { context: context as unknown as CanvasRenderingContext2D, fills }
}
