import { describe, expect, it } from "vitest"

import type { AutomationTarget } from "@/bindings"
import { buildProject, demoProject } from "@/lib/ipc/sim/project"
import { targetState } from "@/lib/automation/targets"

import {
  formatAutomationValue,
  formatBarBeat,
  parseAutomationValue,
} from "./format"

function song() {
  const ids = { synth: 0, track: 0, reverb: 0 }
  const base = demoProject()
  const project = buildProject(base, (run) => {
    ids.track = base.mixer.tracks[1].id
    ;[ids.synth] = run({ type: "addChannel", instrument: "subtractiveSynth" })
    ;[ids.reverb] = run({ type: "addEffect", track: ids.track, kind: "reverb" })
  })
  /** The index of a synth setting, by its id. */
  const synthParam = (id: string) => {
    for (let param = 0; param < 60; param += 1) {
      const state = targetState(project, {
        type: "instrumentParam",
        channel: ids.synth,
        param,
      })
      if (state?.info?.id === id) return param
    }
    throw new Error(`no setting ${id}`)
  }
  return { project, ids, kick: base.channels[0], synthParam }
}

describe("formatAutomationValue", () => {
  it("shows a volume in decibels", () => {
    const { project, ids, kick } = song()
    const fader: AutomationTarget = { type: "trackVolume", track: ids.track }
    expect(formatAutomationValue(project, fader, Math.SQRT1_2)).toBe("0.0 dB")
    expect(formatAutomationValue(project, fader, 1)).toBe("+6.0 dB")
    expect(formatAutomationValue(project, fader, 0.5)).toBe("−6.0 dB")
    expect(formatAutomationValue(project, fader, 0)).toBe("−∞ dB")
    expect(
      formatAutomationValue(
        project,
        { type: "channelVolume", channel: kick.id },
        Math.SQRT1_2
      )
    ).toBe("0.0 dB")
  })

  it("shows a pan as left and right, a mix as a share and the tempo in bpm", () => {
    const { project, ids } = song()
    const pan: AutomationTarget = { type: "trackPan", track: ids.track }
    expect(formatAutomationValue(project, pan, 0.5)).toBe("C")
    expect(formatAutomationValue(project, pan, 0)).toBe("L100")
    expect(formatAutomationValue(project, pan, 0.75)).toBe("R50")
    expect(
      formatAutomationValue(
        project,
        { type: "effectMix", track: ids.track, effect: ids.reverb },
        0.4
      )
    ).toBe("40%")
    expect(formatAutomationValue(project, { type: "tempo" }, 0.21484375)).toBe(
      "120.00 bpm"
    )
  })

  it("shows a setting the way its own knob does", () => {
    const { project, ids, synthParam } = song()
    const cutoff: AutomationTarget = {
      type: "instrumentParam",
      channel: ids.synth,
      param: synthParam("filter.cutoffHz"),
    }
    // Half way up a range of 20 Hz to 20 kHz, in equal ratios.
    expect(formatAutomationValue(project, cutoff, 0.5)).toBe("632 Hz")
    expect(formatAutomationValue(project, cutoff, 1)).toBe("20.0 kHz")
    const attack: AutomationTarget = {
      type: "instrumentParam",
      channel: ids.synth,
      param: synthParam("ampEnvelope.attackMs"),
    }
    expect(formatAutomationValue(project, attack, 0)).toMatch(/ms$/)
  })

  it("is empty for a target that is gone", () => {
    const { project } = song()
    expect(
      formatAutomationValue(project, { type: "trackPan", track: 9999 }, 0.5)
    ).toBe("")
  })
})

describe("parseAutomationValue", () => {
  it("reads what was typed in the target's own unit", () => {
    const { project, ids, synthParam } = song()
    const fader: AutomationTarget = { type: "trackVolume", track: ids.track }
    expect(parseAutomationValue(project, fader, "0")).toBeCloseTo(
      Math.SQRT1_2,
      6
    )
    expect(parseAutomationValue(project, fader, "-6 dB")).toBeCloseTo(0.5, 2)
    expect(parseAutomationValue(project, { type: "tempo" }, "120 bpm")).toBe(
      0.21484375
    )
    expect(
      parseAutomationValue(
        project,
        { type: "trackPan", track: ids.track },
        "R50"
      )
    ).toBe(0.75)
    const cutoff: AutomationTarget = {
      type: "instrumentParam",
      channel: ids.synth,
      param: synthParam("filter.cutoffHz"),
    }
    expect(parseAutomationValue(project, cutoff, "632.46")).toBeCloseTo(0.5, 3)
    expect(parseAutomationValue(project, cutoff, "2k")).toBeCloseTo(
      Math.log(100) / Math.log(1000),
      4
    )
  })

  it("goes back to what it printed", () => {
    const { project, ids } = song()
    const targets: AutomationTarget[] = [
      { type: "trackVolume", track: ids.track },
      { type: "trackPan", track: ids.track },
      { type: "effectMix", track: ids.track, effect: ids.reverb },
      { type: "tempo" },
    ]
    for (const target of targets) {
      for (const value of [0.1, 0.4, 0.75, 1]) {
        const text = formatAutomationValue(project, target, value)
        expect(
          parseAutomationValue(project, target, text),
          `${target.type} ${text}`
        ).toBeCloseTo(value, 2)
      }
    }
  })

  it("is null for text that is not a value, and for a target that is gone", () => {
    const { project, ids } = song()
    const fader: AutomationTarget = { type: "trackVolume", track: ids.track }
    expect(parseAutomationValue(project, fader, "loud")).toBeNull()
    expect(parseAutomationValue(project, { type: "tempo" }, "")).toBeNull()
    expect(
      parseAutomationValue(project, { type: "trackPan", track: 9999 }, "C")
    ).toBeNull()
  })
})

describe("formatBarBeat", () => {
  it("counts bars, beats and steps from 1", () => {
    const four = { numerator: 4, denominator: 4 }
    expect(formatBarBeat(0, four)).toBe("1.1.1")
    expect(formatBarBeat(3840 + 960 + 240, four)).toBe("2.2.2")
    expect(formatBarBeat(3 * 3840 - 1, four)).toBe("3.4.4")
  })
})
