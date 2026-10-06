import { describe, expect, it } from "vitest"

import { isSoftwareGpu } from "./renderer"

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
