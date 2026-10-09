import { describe, expect, it } from "vitest"

import type { BrowserEntry, BrowserRoot } from "@/bindings"

import { loadedFolderIds } from "./folder-expand"
import { rowId, type Listing } from "./tree-model"

const ROOT: BrowserRoot = { name: "Sounds", path: "/sounds", kind: "user" }
const folder = (path: string): BrowserEntry => ({
  name: path.split("/").pop() ?? path,
  path,
  kind: "folder",
})

describe("loadedFolderIds", () => {
  it("includes the root and nested folders from ready listings", () => {
    expect(
      loadedFolderIds([ROOT], {
        "/sounds": { status: "ready", entries: [folder("/sounds/Drums")] },
        "/sounds/Drums": {
          status: "ready",
          entries: [folder("/sounds/Drums/Kicks")],
        },
      })
    ).toEqual(
      new Set([
        rowId(ROOT.path, ROOT.path),
        rowId(ROOT.path, "/sounds/Drums"),
        rowId(ROOT.path, "/sounds/Drums/Kicks"),
      ])
    )
  })

  it("includes a folder under every root it belongs to", () => {
    const inner: BrowserRoot = {
      name: "Drums",
      path: "/sounds/Drums",
      kind: "user",
    }
    expect(
      loadedFolderIds([ROOT, inner], {
        "/sounds": { status: "ready", entries: [folder(inner.path)] },
        [inner.path]: {
          status: "ready",
          entries: [folder("/sounds/Drums/Kicks")],
        },
      })
    ).toEqual(
      new Set([
        rowId(ROOT.path, ROOT.path),
        rowId(inner.path, inner.path),
        rowId(ROOT.path, inner.path),
        rowId(ROOT.path, "/sounds/Drums/Kicks"),
        rowId(inner.path, "/sounds/Drums/Kicks"),
      ])
    )
  })

  it.each<Listing>([
    { status: "loading", entries: [folder("/sounds/Drums")] },
    { status: "error", message: "Access denied" },
  ])("includes only the root when a listing is $status", (listing) => {
    expect(loadedFolderIds([ROOT], { [ROOT.path]: listing })).toEqual(
      new Set([rowId(ROOT.path, ROOT.path)])
    )
  })

  it.each<BrowserEntry["kind"]>(["audio", "project", "other"])(
    "excludes %s file entries",
    (kind) => {
      expect(
        loadedFolderIds([ROOT], {
          [ROOT.path]: {
            status: "ready",
            entries: [{ name: "file", path: "/sounds/file", kind }],
          },
        })
      ).toEqual(new Set([rowId(ROOT.path, ROOT.path)]))
    }
  )

  it("uses only listed folders inside the roots without inventing ancestors", () => {
    expect(
      loadedFolderIds([ROOT], {
        "/sounds/unlisted": {
          status: "ready",
          entries: [
            folder("/sounds/unlisted/known"),
            folder("/sounds-other/Drums"),
            folder("/elsewhere"),
          ],
        },
        "/missing": undefined,
      })
    ).toEqual(
      new Set([
        rowId(ROOT.path, ROOT.path),
        rowId(ROOT.path, "/sounds/unlisted/known"),
      ])
    )
  })
})
