import { describe, expect, it } from "vitest"

import { createPointerFrame } from "./pointer-frame"

/** A grid whose place in the window the test moves. */
function grid() {
  const place = { left: 100, top: 168 }
  const frame = createPointerFrame((event) => ({
    x: event.clientX - place.left,
    y: event.clientY - place.top,
  }))
  return { place, frame }
}

describe("createPointerFrame", () => {
  it("follows the grid while no button is held", () => {
    const { place, frame } = grid()
    expect(frame.point({ clientX: 150, clientY: 200 })).toEqual({ x: 50, y: 32 })
    place.top = 204
    expect(frame.point({ clientX: 150, clientY: 200 })).toEqual({ x: 50, y: -4 })
  })

  it("keeps the frame of the press when the grid moves under a held pointer", () => {
    const { place, frame } = grid()
    const press = { clientX: 150, clientY: 200 }
    frame.hold(press)
    expect(frame.held).toBe(true)
    expect(frame.point(press)).toEqual({ x: 50, y: 32 })

    // A strip appears above the grid and pushes it 36 pixels down.
    place.top = 204
    expect(frame.point(press)).toEqual({ x: 50, y: 32 })
    expect(frame.point({ clientX: 190, clientY: 200 })).toEqual({ x: 90, y: 32 })

    frame.release()
    expect(frame.held).toBe(false)
    expect(frame.point(press)).toEqual({ x: 50, y: -4 })
  })
})
