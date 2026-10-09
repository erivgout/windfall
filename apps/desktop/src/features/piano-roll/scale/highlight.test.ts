import { describe, expect, it, vi } from "vitest"

import { deviceTransform, type OverlayFrame } from "@/lib/canvas"

import { TEST_VIEWPORT } from "../test-utils"
import { scaleHighlightPainter } from "./highlight"

function canvas() {
  const context = {
    save: vi.fn(),
    restore: vi.fn(),
    fillRect: vi.fn(),
    fillStyle: "",
  }
  return { context, ctx: context as unknown as CanvasRenderingContext2D }
}

function frame(dpr = 1): OverlayFrame {
  const viewport = { ...TEST_VIEWPORT, dpr }
  return {
    viewport,
    transform: deviceTransform(viewport),
    // The painter deliberately does not depend on a theme or on note data.
    theme: {} as OverlayFrame["theme"],
  }
}

describe("scale highlight overlay", () => {
  it("leaves the canvas unchanged when Off", () => {
    const { ctx, context } = canvas()
    scaleHighlightPainter({ root: 0, scale: "off" })(ctx, frame())
    expect(context.save).not.toHaveBeenCalled()
    expect(context.fillRect).not.toHaveBeenCalled()
  })

  it.each([1, 1.5, 2])(
    "dims C# but not C, aligned at device density %s",
    (dpr) => {
      const { ctx, context } = canvas()
      const current = frame(dpr)
      scaleHighlightPainter({ root: 0, scale: "major" })(ctx, current)
      expect(context.fillRect).toHaveBeenCalledWith(
        0,
        144 * dpr,
        800 * dpr,
        16 * dpr
      )
      expect(
        context.fillRect.mock.calls.some((call) => call[1] === 160 * dpr)
      ).toBe(false)
      for (const call of context.fillRect.mock.calls) {
        expect(call.every(Number.isFinite)).toBe(true)
      }
      expect(context.fillStyle).toBe("rgba(0, 0, 0, 0.18)")
      expect(context.restore).toHaveBeenCalledOnce()
    }
  )
})
