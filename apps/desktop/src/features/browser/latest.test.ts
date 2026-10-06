import { describe, expect, it } from "vitest"

import { formatChannels, formatDuration, formatSampleRate } from "./format"
import { createLatestRunner } from "./latest"
import { deferred } from "./testing"

async function turn() {
  for (let count = 0; count < 5; count += 1) await Promise.resolve()
}

describe("createLatestRunner", () => {
  it("starts a lone job in the same call", () => {
    const started: string[] = []
    const runner = createLatestRunner<string>(async (job) => {
      started.push(job)
    })
    runner.request("a")
    expect(started).toEqual(["a"])
  })

  it("runs one job at a time and skips all but the newest waiting one", async () => {
    const started: string[] = []
    const gates = new Map<string, ReturnType<typeof deferred<void>>>()
    const runner = createLatestRunner<string>((job) => {
      started.push(job)
      const gate = deferred<void>()
      gates.set(job, gate)
      return gate.promise
    })

    runner.request("a")
    runner.request("b")
    runner.request("c")
    runner.request("d")
    expect(started).toEqual(["a"])

    gates.get("a")?.resolve()
    await turn()
    expect(started).toEqual(["a", "d"])

    gates.get("d")?.resolve()
    await turn()
    runner.request("e")
    expect(started).toEqual(["a", "d", "e"])
  })

  it("tells a running job when a newer one is waiting", async () => {
    const gate = deferred<void>()
    let check: () => boolean = () => false
    const runner = createLatestRunner<string>((job, superseded) => {
      if (job === "a") check = superseded
      return job === "a" ? gate.promise : Promise.resolve()
    })
    runner.request("a")
    expect(check()).toBe(false)
    runner.request("b")
    expect(check()).toBe(true)
    gate.resolve()
    await turn()
  })

  it("can forget the waiting job", async () => {
    const started: string[] = []
    const gate = deferred<void>()
    const runner = createLatestRunner<string>((job) => {
      started.push(job)
      return gate.promise
    })
    runner.request("a")
    runner.request("b")
    runner.cancel()
    gate.resolve()
    await turn()
    expect(started).toEqual(["a"])
  })

  it("keeps going after a job fails", async () => {
    const started: string[] = []
    const runner = createLatestRunner<string>((job) => {
      started.push(job)
      return job === "a" ? Promise.reject(new Error("no")) : Promise.resolve()
    })
    runner.request("a")
    runner.request("b")
    await turn()
    expect(started).toEqual(["a", "b"])
  })
})

describe("formats", () => {
  it("writes a sample's length", () => {
    expect(formatDuration(0.42)).toBe("0.42 s")
    expect(formatDuration(12.34)).toBe("12.3 s")
    expect(formatDuration(67.4)).toBe("1:07")
    expect(formatDuration(Number.NaN)).toBe("0.00 s")
  })

  it("writes a sample rate", () => {
    expect(formatSampleRate(44_100)).toBe("44.1 kHz")
    expect(formatSampleRate(48_000)).toBe("48 kHz")
  })

  it("writes a channel count", () => {
    expect(formatChannels(1)).toBe("Mono")
    expect(formatChannels(2)).toBe("Stereo")
    expect(formatChannels(6)).toBe("6 channels")
  })
})
