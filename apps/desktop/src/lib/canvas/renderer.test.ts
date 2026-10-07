import { describe, expect, it } from "vitest"

import { isSoftwareGpu, resizedSpan } from "./renderer"

describe("isSoftwareGpu", () => {
  it("recognizes CPU rasterizers", () => {
    for (const name of [
      "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)",
      "llvmpipe (LLVM 15.0.7, 256 bits)",
      "Microsoft Basic Render Driver",
      "Apple Software Renderer",
    ]) {
      expect(isSoftwareGpu(name)).toBe(true)
    }
  })

  it("accepts real GPUs", () => {
    for (const name of [
      "ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 SUPER (0x00002783) Direct3D11 vs_5_0 ps_5_0, D3D11)",
      "ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0, D3D11)",
      "Apple M2",
      "Mesa Intel(R) Xe Graphics (TGL GT2)",
    ]) {
      expect(isSoftwareGpu(name)).toBe(false)
    }
  })
})

describe("resizedSpan", () => {
  it("changes nothing without a resize", () => {
    expect(resizedSpan(960, 480, 0, 0, 0)).toEqual({ start: 960, length: 480 })
    expect(resizedSpan(960, 480, 0, 0, 240)).toEqual({
      start: 960,
      length: 480,
    })
  })

  it("moves the end edge and keeps the start", () => {
    expect(resizedSpan(960, 480, 0, 240, 1)).toEqual({
      start: 960,
      length: 720,
    })
    expect(resizedSpan(960, 480, 0, -240, 1)).toEqual({
      start: 960,
      length: 240,
    })
  })

  it("moves the start edge and keeps the end", () => {
    expect(resizedSpan(960, 480, 240, 0, 1)).toEqual({
      start: 1200,
      length: 240,
    })
    expect(resizedSpan(960, 480, -240, 0, 1)).toEqual({
      start: 720,
      length: 720,
    })
  })

  it("never shrinks below the minimum length", () => {
    expect(resizedSpan(960, 480, 0, -9999, 120)).toEqual({
      start: 960,
      length: 120,
    })
    expect(resizedSpan(960, 480, 9999, 0, 120)).toEqual({
      start: 1320,
      length: 120,
    })
  })

  it("leaves a rect that is already shorter than the minimum as it is", () => {
    expect(resizedSpan(960, 60, 0, -240, 240)).toEqual({
      start: 960,
      length: 60,
    })
    expect(resizedSpan(960, 60, 240, 0, 240)).toEqual({
      start: 960,
      length: 60,
    })
    // It can still grow.
    expect(resizedSpan(960, 60, 0, 240, 240)).toEqual({
      start: 960,
      length: 300,
    })
    expect(resizedSpan(960, 60, -240, 0, 240)).toEqual({
      start: 720,
      length: 300,
    })
  })
})
