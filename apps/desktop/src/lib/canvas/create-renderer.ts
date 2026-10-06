import {
  RendererUnavailableError,
  type RectRenderer,
  type RendererKind,
} from "./renderer"
import { createCanvas2DRenderer } from "./renderer-canvas2d"
import { createWebGL2Renderer } from "./renderer-webgl2"
import { createWebGPURenderer } from "./renderer-webgpu"

export type RendererChoice = RendererKind | "auto"

/**
 * What "auto" tries, in order. WebGL2 is only taken when it runs on a real
 * GPU: on a software rasterizer Canvas 2D is several times faster. See
 * docs/perf/canvas-10k-notes.md.
 */
export const AUTO_RENDERER_ORDER: readonly RendererKind[] = [
  "webgl2",
  "canvas2d",
]

export interface CreatedRenderer {
  readonly renderer: RectRenderer
  readonly canvas: HTMLCanvasElement
}

async function createOne(
  kind: RendererKind,
  canvas: HTMLCanvasElement,
  rejectSoftware: boolean
): Promise<RectRenderer> {
  switch (kind) {
    case "canvas2d":
      return createCanvas2DRenderer(canvas)
    case "webgl2":
      return createWebGL2Renderer(canvas, { rejectSoftware })
    case "webgpu":
      return createWebGPURenderer(canvas)
    default: {
      const _exhaustive: never = kind
      return _exhaustive
    }
  }
}

/**
 * Creates a renderer on a fresh canvas. A canvas is bound to the first
 * context type requested from it, so every attempt needs its own.
 */
export async function createRenderer(
  choice: RendererChoice,
  makeCanvas: () => HTMLCanvasElement
): Promise<CreatedRenderer> {
  const kinds = choice === "auto" ? AUTO_RENDERER_ORDER : [choice]
  let failure: unknown = null
  for (const kind of kinds) {
    const canvas = makeCanvas()
    try {
      return {
        renderer: await createOne(kind, canvas, choice === "auto"),
        canvas,
      }
    } catch (error) {
      failure = error
    }
  }
  if (failure instanceof Error) throw failure
  throw new RendererUnavailableError(kinds[0] ?? "canvas2d", "no renderer")
}
