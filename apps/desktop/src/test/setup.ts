import "@testing-library/jest-dom/vitest"
import { cleanup } from "@testing-library/react"
import { afterEach } from "vitest"

// jsdom leaves out a few browser APIs the UI components call.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub

window.matchMedia ??= (query: string): MediaQueryList => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener() {},
  removeEventListener() {},
  addListener() {},
  removeListener() {},
  dispatchEvent: () => false,
})

// jsdom has no canvas and logs an error for every `getContext`. Components
// already cope with a browser that hands out no context, so this hands out
// none: canvases mount and draw nothing.
HTMLCanvasElement.prototype.getContext = () => null

Element.prototype.scrollIntoView ??= () => {}
Element.prototype.setPointerCapture ??= () => {}
Element.prototype.releasePointerCapture ??= () => {}
Element.prototype.hasPointerCapture ??= () => false

afterEach(() => {
  cleanup()
  localStorage.clear()
})
