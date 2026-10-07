import { describe, expect, it } from "vitest"
import {
  processingCommand,
  fitRatio,
  beatFitRatio,
  type AudioClip,
} from "./processing"
import { naturalTicks, filePosition } from "./geometry"
const clip: AudioClip = {
  id: 1,
  track: 2,
  start: 100,
  length: 960,
  offset: 480,
  muted: false,
  content: {
    type: "audio",
    sample: 3,
    mixerTrack: 0,
    gain: 1,
    pan: 0,
    fadeIn: 120,
    fadeOut: 240,
    reverse: false,
    pitch: 12,
  },
}
const spectral = {
  mode: "spectral",
  ratio: 1.5,
  quality: "high",
  formants: true,
} as const
it("preserves source trims and fades switching tape to independent duration", () => {
  expect(processingCommand([clip], spectral, 12)).toEqual({
    type: "batch",
    label: "Process audio clips",
    commands: [
      {
        type: "updateClips",
        updates: [{ id: 1, patch: { length: 2880, offset: 1440 } }],
      },
      {
        type: "updateAudioClips",
        updates: [
          {
            id: 1,
            patch: { stretch: spectral, pitch: 12, fadeIn: 360, fadeOut: 720 },
          },
        ],
      },
    ],
  })
  const independent = {
    ...clip,
    content: { ...clip.content, stretch: spectral },
  }
  expect(processingCommand([independent], spectral, -12)).toMatchObject({
    commands: [
      { updates: [{ patch: { length: 960, offset: 480 } }] },
      { updates: [{ patch: { pitch: -12, stretch: spectral } }] },
    ],
  })
})
it("maps original waveforms to stretched time with independent pitch and reverse", () => {
  const timing = {
    start: 0,
    length: 5760,
    offset: 0,
    pitch: 12,
    reverse: false,
    stretch: spectral,
  }
  expect(naturalTicks(2, timing, 120)).toBe(5760)
  expect(filePosition(timing, 2880, 2, 120)).toBeCloseTo(0.5)
  expect(filePosition({ ...timing, reverse: true }, 1440, 2, 120)).toBeCloseTo(
    0.75
  )
})
describe("tempo fit", () => {
  it("fits source BPM or the whole source beat length", () => {
    expect(fitRatio(90, 120)).toBe(0.75)
    expect(beatFitRatio(4, 2, 100)).toBe(1.2)
  })
  it("rejects invalid and unsupported values instead of clamping", () => {
    for (const bpm of [0, -1, NaN, 10, 1000])
      expect(() => fitRatio(bpm, 120)).toThrow()
    expect(() => beatFitRatio(4, 0, 120)).toThrow()
    expect(() =>
      processingCommand([clip], { ...spectral, ratio: 5 }, 0)
    ).toThrow()
    expect(() => processingCommand([clip], spectral, 25)).toThrow()
  })
})
