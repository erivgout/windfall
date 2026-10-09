import { describe, expect, it } from "vitest"

import { projectNameChange } from "./project-name"

describe("projectNameChange", () => {
  it("trims surrounding spaces", () => {
    expect(projectNameChange("Song", "  New song  ")).toEqual({
      ok: true,
      name: "New song",
    })
  })

  it("returns no new name when unchanged after trimming", () => {
    expect(projectNameChange("Song", "  Song  ")).toEqual({
      ok: true,
      name: null,
    })
  })

  it("rejects an empty draft", () => {
    expect(projectNameChange("Song", " \n\t ")).toEqual({
      ok: false,
      reason: "empty",
    })
  })

  it("accepts 256 UTF-8 bytes and rejects 257", () => {
    const name = "é".repeat(128)
    expect(projectNameChange("Song", ` ${name} `)).toEqual({ ok: true, name })
    expect(projectNameChange("Song", name + "x")).toEqual({
      ok: false,
      reason: "long",
    })
  })

  it("returns a new name by itself", () => {
    expect(projectNameChange("Song", "New song")).toEqual({
      ok: true,
      name: "New song",
    })
  })
})
