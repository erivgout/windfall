import { fireEvent } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it } from "vitest"

import { installWebviewGuard } from "./webview-guard"

let uninstall: () => void

beforeEach(() => {
  document.body.innerHTML =
    "<div id='canvas'></div><input id='field' /><textarea id='area'></textarea>"
  uninstall = installWebviewGuard()
})
afterEach(() => uninstall())

const element = (id: string) => {
  const found = document.getElementById(id)
  if (!found) throw new Error(`no #${id}`)
  return found
}
/** True when the webview would have acted on the key. */
const reachesWebview = (init: KeyboardEventInit, target = document.body) =>
  fireEvent.keyDown(target, init)

describe("webview guard", () => {
  it("blocks the webview's own menu, except in a text field", () => {
    expect(fireEvent.contextMenu(element("canvas"))).toBe(false)
    expect(fireEvent.contextMenu(document.body)).toBe(false)
    expect(fireEvent.contextMenu(element("field"))).toBe(true)
    expect(fireEvent.contextMenu(element("area"))).toBe(true)
  })

  it("swallows reload, find, print, zoom and the other browser keys", () => {
    for (const init of [
      { key: "F5", code: "F5" },
      { key: "F5", code: "F5", ctrlKey: true },
      { key: "r", code: "KeyR", ctrlKey: true },
      { key: "R", code: "KeyR", ctrlKey: true, shiftKey: true },
      { key: "F3", code: "F3" },
      { key: "F7", code: "F7" },
      { key: "p", code: "KeyP", ctrlKey: true },
      { key: "g", code: "KeyG", ctrlKey: true },
      { key: "j", code: "KeyJ", ctrlKey: true },
      { key: "u", code: "KeyU", ctrlKey: true },
      { key: "=", code: "Equal", ctrlKey: true },
      { key: "+", code: "Equal", ctrlKey: true, shiftKey: true },
      { key: "-", code: "Minus", ctrlKey: true },
      { key: "0", code: "Digit0", ctrlKey: true },
      { key: "+", code: "NumpadAdd", ctrlKey: true },
      { key: "ArrowLeft", code: "ArrowLeft", altKey: true },
      { key: "ArrowRight", code: "ArrowRight", altKey: true },
      { key: "r", code: "KeyR", metaKey: true },
    ]) {
      expect(reachesWebview(init), JSON.stringify(init)).toBe(false)
    }
  })

  it("leaves every other key alone", () => {
    for (const init of [
      { key: "a", code: "KeyA" },
      { key: "r", code: "KeyR" },
      { key: "s", code: "KeyS", ctrlKey: true },
      { key: "Delete", code: "Delete" },
      { key: "ArrowLeft", code: "ArrowLeft" },
      { key: "F2", code: "F2" },
      { key: " ", code: "Space" },
    ]) {
      expect(reachesWebview(init), JSON.stringify(init)).toBe(true)
    }
  })

  it("stops Backspace from going back a page, but not from deleting text", () => {
    const backspace = { key: "Backspace", code: "Backspace" }
    expect(reachesWebview(backspace)).toBe(false)
    expect(reachesWebview(backspace, element("field"))).toBe(true)
  })

  it("stops Ctrl+wheel from zooming the page, and leaves plain scrolling", () => {
    expect(fireEvent.wheel(element("canvas"), { ctrlKey: true })).toBe(false)
    expect(fireEvent.wheel(element("canvas"))).toBe(true)
  })

  it("lets everything through again once it is removed", () => {
    uninstall()
    expect(fireEvent.contextMenu(element("canvas"))).toBe(true)
    expect(reachesWebview({ key: "F5", code: "F5" })).toBe(true)
  })
})
