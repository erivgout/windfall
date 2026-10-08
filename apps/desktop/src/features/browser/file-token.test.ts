import { beforeEach, expect, it } from "vitest"

import type { LibraryFileToken } from "@/bindings"

import { acceptFile, type FileRequest } from "./file-token"
import { invalidateLibrary, useLibraryStore } from "./library-store"
import { resetBrowserStore, useBrowserStore } from "./store"

beforeEach(() => {
  resetBrowserStore()
  useBrowserStore.setState({
    roots: ["/sounds", "/sounds/kits", "/sounds/kits-copy"].map((path) => ({
      path,
      name: path,
      kind: "user",
    })),
  })
})

const path = "/sounds/kits/tone.wav"
const token: LibraryFileToken = {
  path,
  rootPath: "/sounds",
  generation: 7,
  fingerprint: "version",
}
const request = (): FileRequest => ({
  path,
  rootPath: "/sounds/kits",
  epoch: useLibraryStore.getState().epoch,
})

it("accepts the canonical containing root without changing the captured tree root", () => {
  const captured = Object.freeze(request())
  expect(acceptFile(captured, token)).toBe(token)
  expect(captured.rootPath).toBe("/sounds/kits")
})

it.each(["/", "/sound", "/sounds/kits-copy"])(
  "refuses an unconfigured or non-containing canonical root %s",
  (rootPath) =>
    expect(() => acceptFile(request(), { ...token, rootPath })).toThrow(
      "out of date"
    )
)

it("refuses a configured selection root which does not contain the file", () => {
  expect(() =>
    acceptFile({ ...request(), rootPath: "/sounds/kits-copy" }, token)
  ).toThrow("out of date")
})

it("never accepts a canonical outer-root token across selection invalidation", () => {
  const captured = request()
  invalidateLibrary()
  expect(() => acceptFile(captured, token)).toThrow("out of date")
})

it("refuses a different file even when both roots overlap", () => {
  expect(() =>
    acceptFile(request(), { ...token, path: "/sounds/kits/other.wav" })
  ).toThrow("out of date")
})
