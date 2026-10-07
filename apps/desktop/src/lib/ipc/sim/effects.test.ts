import { describe, expect, it } from "vitest"

import type { Command } from "@/bindings"

import { simulatedGainReductions, simulatedLatencyFrames } from "./effects"
import { buildProject, demoProject } from "./project"
import { TransportSim } from "./transport"

type Run = (command: Command) => number[]

const build = (edit: (run: Run) => void) => buildProject(demoProject(), edit)

describe("simulated latency", () => {
  it("is nothing for samplers with no effects", () => {
    expect(simulatedLatencyFrames(demoProject(), 48_000)).toBe(0)
  })

  it("counts the synth's own delay", () => {
    const project = build((run) => {
      run({ type: "addChannel", instrument: "subtractiveSynth" })
    })
    expect(simulatedLatencyFrames(project, 48_000)).toBe(12)
  })

  it("adds a limiter's look-ahead along the way to the master", () => {
    const project = build((run) => {
      const [, track] = run({
        type: "addChannel",
        instrument: "subtractiveSynth",
      })
      // 5 ms on the synth's track and 5 ms on the master.
      run({ type: "addEffect", track, kind: "limiter" })
      run({ type: "addEffect", track: 0, kind: "limiter" })
      run({ type: "addEffect", track: 0, kind: "reverb" })
    })
    expect(simulatedLatencyFrames(project, 48_000)).toBe(12 + 240 + 240)
    expect(simulatedLatencyFrames(project, 44_100)).toBe(12 + 221 + 221)
  })
})

describe("simulated gain reduction", () => {
  const project = build((run) => {
    run({ type: "addEffect", track: 0, kind: "eq" })
    run({ type: "addEffect", track: 0, kind: "compressor" })
    run({ type: "addEffect", track: 0, kind: "limiter" })
    const [track] = run({ type: "addMixerTrack" })
    run({ type: "addEffect", track, kind: "compressor" })
  })
  const ids = (index: number) =>
    project.mixer.tracks[index].effects.map((slot) => slot.id)

  it("lists every compressor and limiter in mixer order, then chain order", () => {
    const readings = simulatedGainReductions(project, () => 0)
    const [, compressor, limiter] = ids(0)
    const last = project.mixer.tracks.length - 1
    expect(readings.map((reading) => reading.effect)).toEqual([
      compressor,
      limiter,
      ...ids(last),
    ])
    expect(readings.every((reading) => reading.db === 0)).toBe(true)
  })

  it("turns down what is over the threshold and the ceiling", () => {
    // Full scale on the master: 18 dB over a 4:1 threshold, 0.3 over the
    // limiter's ceiling. Silence everywhere else.
    const readings = simulatedGainReductions(project, (track) =>
      track === 0 ? 1 : 0
    )
    expect(readings[0].db).toBeCloseTo(13.5, 5)
    expect(readings[1].db).toBeCloseTo(0.3, 5)
    expect(readings[2].db).toBe(0)
  })

  it("is part of every frame the transport reports", () => {
    const transport = new TransportSim(project.patterns[0].id)
    const frame = transport.advance(project, 0.01)
    expect(frame.gainReductions).toHaveLength(3)
  })
})

describe("notes played by hand", () => {
  const project = build((run) => {
    run({ type: "addChannel", instrument: "subtractiveSynth" })
  })
  const synth = project.channels.at(-1)!
  const kick = project.channels[0]
  const place = (track: number) =>
    project.mixer.tracks.findIndex((item) => item.id === track) * 2

  it("sound for as long as an instrument's key is held", () => {
    const transport = new TransportSim(project.patterns[0].id)
    const level = () =>
      transport.advance(project, 0.2).meters[place(synth.mixerTrack)]

    expect(level()).toBe(0)
    transport.noteOn(synth.id, 60, 1)
    const held = level()
    expect(held).toBeGreaterThan(0.1)
    expect(level()).toBe(held)
    expect(level()).toBe(held)

    transport.noteOff(synth.id, 60)
    expect(level()).toBeLessThan(held)
    for (let frame = 0; frame < 20; frame += 1) level()
    expect(level()).toBe(0)
  })

  it("keep sounding until the last of two keys comes up", () => {
    const transport = new TransportSim(project.patterns[0].id)
    const level = () =>
      transport.advance(project, 0.2).meters[place(synth.mixerTrack)]
    transport.noteOn(synth.id, 60, 0.5)
    transport.noteOn(synth.id, 64, 1)
    const both = level()
    transport.noteOff(synth.id, 64)
    const one = level()
    expect(one).toBeGreaterThan(0)
    expect(one).toBeLessThan(both)
    transport.noteOff(synth.id, 60)
    // A key that was never down is ignored.
    transport.noteOff(synth.id, 72)
    for (let frame = 0; frame < 20; frame += 1) level()
    expect(level()).toBe(0)
  })

  it("hit a sampler once, however long the key is held", () => {
    const transport = new TransportSim(project.patterns[0].id)
    const level = () =>
      transport.advance(project, 0.2).meters[place(kick.mixerTrack)]
    transport.noteOn(kick.id, 60, 1)
    const hit = level()
    expect(hit).toBeGreaterThan(0)
    expect(level()).toBeLessThan(hit)
  })
})
