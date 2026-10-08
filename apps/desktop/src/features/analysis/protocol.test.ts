import { describe, expect, it, vi } from "vitest"
import { invoke } from "@tauri-apps/api/core"
import { createTauriBackend } from "@/lib/ipc/tauri"
import { decimal, validRange } from "./types"
import { unavailableAnalysis } from "./unavailable"
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), Channel: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }))
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: vi.fn() }))
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }))

describe("analysis native protocol", () => {
  it("preserves exact decimal identifiers and forwards refusals without successful fallback", async () => {
    const native = createTauriBackend()
    vi.mocked(invoke).mockResolvedValue(null)
    const request = {
      ticket: "9007199254740993",
      request: "18446744073709551614",
      replaceOriginal: false,
    }
    await native.analysisApply(request)
    expect(invoke).toHaveBeenLastCalledWith("analysis_apply", { request })
    await native.analysisReview(request.ticket)
    expect(invoke).toHaveBeenLastCalledWith("analysis_review", {
      ticket: request.ticket,
    })
    await native.analysisRetryCleanup(request.ticket)
    expect(invoke).toHaveBeenLastCalledWith("analysis_retry_cleanup", {
      job: request.ticket,
    })
    vi.mocked(invoke).mockRejectedValue(
      "analysis:unavailable: No inference algorithm is installed."
    )
    await expect(native.analysisStatus(request.ticket)).rejects.toThrow(
      "No inference algorithm"
    )
  })
  it("refuses browser inference, import, review and apply honestly", async () => {
    const browser = unavailableAnalysis()
    expect(await browser.analysisCapability()).toMatchObject({
      native: false,
      available: false,
      models: [],
    })
    await expect(browser.analysisStatus("1")).rejects.toThrow("desktop app")
    await expect(browser.analysisReview("1")).rejects.toThrow("desktop app")
    await expect(
      browser.analysisApply({
        ticket: "1",
        request: "2",
        replaceOriginal: true,
      })
    ).rejects.toThrow("desktop app")
  })
  it("checks ranges beyond JS safe integers and refuses aliases/overflow", () => {
    expect(validRange("9007199254740993", "9007199254740994")).toBe(true)
    expect(validRange("9007199254740994", "9007199254740993")).toBe(false)
    for (const value of [
      "01",
      "-1",
      "+1",
      "1.0",
      "18446744073709551616",
      "1e3",
      "",
    ])
      expect(decimal(value)).toBe(false)
    expect(decimal("18446744073709551615")).toBe(true)
  })
})
