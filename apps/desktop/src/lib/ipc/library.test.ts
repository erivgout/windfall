import { afterEach, describe, expect, it } from "vitest"

import type { LibrarySearch } from "@/bindings"
import { TEST_DIALOGS } from "@/test/harness"

import { createMockBackend, type MockBackend } from "./mock"

const opened: MockBackend[] = []
function create(storage: Pick<Storage, "getItem" | "setItem"> | null = null) {
  const backend = createMockBackend({ storage, dialogs: TEST_DIALOGS })
  opened.push(backend)
  return backend
}
afterEach(() => opened.splice(0).forEach((b) => b.dispose()))
const search = (
  query = "",
  favoritesOnly = false,
  tags: string[] = []
): LibrarySearch => ({ query, favoritesOnly, tags })

describe("browser library mock", () => {
  it("shares recursive filename/path, wildcard, Boolean and quoted query semantics", async () => {
    const backend = create()
    await backend.browserAddRoot("/samples/My kit")
    const found = await backend.librarySearch(
      search('DRUMS\\K?CKS AND *.WAV NOT 03 OR "vocal chop"')
    )
    expect(found.entries.map((e) => e.entry.name)).toEqual([
      "Kick 01.wav",
      "Kick 02.wav",
      "Kick Punch.wav",
      "Vocal chop.wav",
    ])
    expect(found.limitation).toContain("requires the desktop app")
    expect(found.entries[0].token.rootPath).toBe("/factory")
    await expect(backend.librarySearch(search("kick OR"))).rejects.toThrow(
      "term"
    )
    await expect(
      backend.librarySearch(search("x".repeat(513)))
    ).rejects.toThrow("512")
  })

  it("persists normalized tags, favorites and roots without touching project history", async () => {
    const values = new Map<string, string>()
    const storage = {
      getItem: (k: string) => values.get(k) ?? null,
      setItem: (k: string, v: string) => {
        values.set(k, v)
      },
    }
    const backend = create(storage)
    await backend.browserAddRoot("/samples/My kit")
    const path = "/samples/My kit/Vocal chop.wav"
    const before = await backend.documentSnapshot()
    const meta = await backend.librarySetMetadata(path, {
      favorite: true,
      tags: [" Warm ", "vocal", "WARM"],
    })
    expect(meta).toEqual({ favorite: true, tags: ["vocal", "warm"] })
    expect(await backend.documentSnapshot()).toEqual(before)
    const reopened = create(storage)
    const found = await reopened.librarySearch(search("CHOP", true, ["WARM"]))
    expect(found.entries).toHaveLength(1)
    expect(found.entries[0].entry.path).toBe(path)
    await reopened.librarySetMetadata(path, { favorite: false, tags: [] })
    expect(
      (await reopened.librarySearch(search("", true))).entries
    ).toHaveLength(0)
  })

  it("rejects removed-root and refreshed-result tokens for audition, facts and all import destinations", async () => {
    const backend = create()
    await backend.browserAddRoot("/samples/My kit")
    const { entry, token } = (
      await backend.librarySearch(search('"vocal chop"'))
    ).entries[0]
    await backend.previewPlay(entry.path, token)
    await backend.sampleInfo(entry.path, token)
    await backend.browserRemoveRoot("/samples/My kit")
    const before = await backend.documentSnapshot()
    await expect(
      backend.addChannelFromFile(entry.path, undefined, token)
    ).rejects.toThrow("out of date")
    await expect(
      backend.setChannelSampleFromFile(
        before.project.channels[0].id,
        entry.path,
        token
      )
    ).rejects.toThrow("out of date")
    await expect(
      backend.addAudioClipFromFile(entry.path, { start: 0 }, token)
    ).rejects.toThrow("out of date")
    await expect(backend.previewPlay(entry.path, token)).rejects.toThrow(
      "out of date"
    )
    expect(await backend.documentSnapshot()).toEqual(before)
    expect((await backend.librarySearch(search("chop"))).entries).toHaveLength(
      0
    )
    const held = (await backend.librarySearch(search("kick"))).entries[0]
    await backend.libraryRefresh()
    await expect(
      backend.sampleInfo(held.entry.path, held.token)
    ).rejects.toThrow("out of date")
  })

  it("preserves damaged metadata and reports write failures without changing in-memory favorites", async () => {
    const path = "/factory/Drums/Kicks/Kick 01.wav"
    const damaged = {
      getItem: (k: string) =>
        k === "windfall.mock.browser-library" ? "broken" : null,
      setItem: () => {
        throw new Error("must not overwrite")
      },
    }
    const backend = create(damaged)
    expect((await backend.librarySearch(search())).issues[0]).toContain(
      "preserved"
    )
    await expect(
      backend.librarySetMetadata(path, { favorite: true, tags: [] })
    ).rejects.toThrow("preserved")
    const full = create({
      getItem: () => null,
      setItem: () => {
        throw new Error("full")
      },
    })
    await expect(
      full.librarySetMetadata(path, { favorite: true, tags: [] })
    ).rejects.toThrow("Could not save")
    expect((await full.librarySearch(search("", true))).entries).toHaveLength(0)
    await expect(full.browserAddRoot("/samples/My kit")).rejects.toThrow(
      "Could not save browser folders"
    )
    expect(await full.browserRoots()).toHaveLength(1)
  })
})
