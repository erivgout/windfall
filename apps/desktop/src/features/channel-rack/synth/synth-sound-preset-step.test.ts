import { describe, expect, it } from "vitest"

import { instrumentDescriptor, paramInfo, writeParam } from "@/features/params"

import { SYNTH_PRESETS } from "./presets"
import { nextSynthSoundPreset } from "./synth-sound-preset-step"

const sounds = SYNTH_PRESETS.map((preset, index) => ({ ...preset, index }))

describe("synth sound preset stepping", () => {
  it("starts at Init and ends at Sub", () => {
    expect(SYNTH_PRESETS[0].name).toBe("Init")
    expect(SYNTH_PRESETS[SYNTH_PRESETS.length - 1].name).toBe("Sub")
  })

  it.each(sounds)(
    "steps previous from $name without wrapping",
    ({ params, index }) => {
      expect(nextSynthSoundPreset(params, "previous")?.id ?? null).toBe(
        SYNTH_PRESETS[index - 1]?.id ?? null
      )
    }
  )

  it.each(sounds)(
    "steps next from $name without wrapping",
    ({ params, index }) => {
      expect(nextSynthSoundPreset(params, "next")?.id ?? null).toBe(
        SYNTH_PRESETS[index + 1]?.id ?? null
      )
    }
  )

  it.each(["previous", "next"] as const)(
    "leaves an edited sound alone when stepping %s",
    (direction) => {
      const init = SYNTH_PRESETS[0].params
      const gain = paramInfo(instrumentDescriptor("subtractiveSynth"), "gain")
      const edited = writeParam(init, gain, init.gain / 2)

      expect(edited).not.toBe(init)
      expect(edited.gain).not.toBe(init.gain)
      expect(nextSynthSoundPreset(edited, direction)).toBeNull()
    }
  )
})
