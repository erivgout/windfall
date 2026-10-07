import { describe, expect, it } from "vitest"

import {
  audioEndTick,
  clampFade,
  describeFade,
  fadeFromPointer,
  fadeGainAt,
  fadeInGain,
  fadeOutGain,
  filePosition,
  fileWindow,
  gainFromDrag,
  naturalTicks,
  speedOf,
  ticksPerSecond,
  wholeClipTicks,
  type AudioTiming,
} from "./geometry"

const BAR = 3840
const BEAT = 960

/** A clip of a two second file, which is one bar at 120 bpm. */
const clip = (more: Partial<AudioTiming> = {}): AudioTiming => ({
  start: BAR,
  length: BAR,
  offset: 0,
  pitch: 0,
  reverse: false,
  ...more,
})

describe("speed and time", () => {
  it("doubles the speed for every octave up", () => {
    expect(speedOf(0)).toBe(1)
    expect(speedOf(12)).toBe(2)
    expect(speedOf(-12)).toBe(0.5)
    expect(speedOf(7)).toBeCloseTo(1.4983, 4)
  })

  it("counts 16 ticks a second for each beat a minute", () => {
    expect(ticksPerSecond(120)).toBe(1920)
    expect(ticksPerSecond(60)).toBe(960)
  })
})

describe("the natural length of a clip", () => {
  it("is the file's length at the tempo, less what the offset skips", () => {
    expect(naturalTicks(2, clip(), 120)).toBe(BAR)
    expect(naturalTicks(2, clip({ offset: BEAT }), 120)).toBe(BAR - BEAT)
    // The same seconds are more ticks at a faster tempo.
    expect(naturalTicks(2, clip(), 240)).toBe(2 * BAR)
  })

  it("halves an octave up and doubles an octave down", () => {
    expect(naturalTicks(2, clip({ pitch: 12 }), 120)).toBe(BAR / 2)
    expect(naturalTicks(2, clip({ pitch: -12 }), 120)).toBe(2 * BAR)
  })

  it("gives a new clip of a whole file a whole number of ticks, at least one", () => {
    expect(wholeClipTicks(3.75, 128)).toBe(7680)
    expect(wholeClipTicks(0.42, 128)).toBe(Math.ceil(0.42 * 128 * 16))
    expect(wholeClipTicks(0, 128)).toBe(1)
  })
})

describe("where the sound stops", () => {
  it("is the clip's end while the audio lasts that long", () => {
    expect(audioEndTick(clip({ length: BEAT }), 2, 120)).toBe(BAR + BEAT)
  })

  it("is sooner when the audio runs out before the clip does", () => {
    expect(audioEndTick(clip({ length: 3 * BAR }), 2, 120)).toBe(2 * BAR)
    // A slower tempo: the same two seconds are fewer ticks.
    expect(audioEndTick(clip({ length: 3 * BAR }), 2, 60)).toBe(BAR + BAR / 2)
    // An offset past the end of the file leaves nothing to play.
    expect(audioEndTick(clip({ offset: 9 * BAR }), 2, 120)).toBe(BAR)
  })
})

describe("the place in the file under a tick", () => {
  it("runs from the start of the file to its end across the audio", () => {
    expect(filePosition(clip(), BAR, 2, 120)).toBe(0)
    expect(filePosition(clip(), BAR + BEAT, 2, 120)).toBeCloseTo(0.25, 12)
    expect(filePosition(clip(), 2 * BAR, 2, 120)).toBeCloseTo(1, 12)
    // Past the end of the audio there is nothing.
    expect(filePosition(clip(), 3 * BAR, 2, 120)).toBeGreaterThan(1)
  })

  it("starts further in by the offset, counted at the stored tempo", () => {
    const trimmed = clip({ start: BAR + BEAT, offset: BEAT })
    // Trimming the left edge left the rest of the audio where it was.
    expect(filePosition(trimmed, BAR + BEAT, 2, 120)).toBeCloseTo(0.25, 12)
    expect(filePosition(trimmed, 2 * BAR, 2, 120)).toBeCloseTo(1, 12)
  })

  it("goes through the file faster when the clip is pitched up", () => {
    const up = clip({ pitch: 12 })
    expect(filePosition(up, BAR + BEAT, 2, 120)).toBeCloseTo(0.5, 12)
    // The offset skips twice as much of the file too.
    expect(
      filePosition(clip({ pitch: 12, offset: BEAT }), BAR, 2, 120)
    ).toBeCloseTo(0.5, 12)
  })

  it("reads a reversed clip from the end of the file", () => {
    const backwards = clip({ reverse: true })
    expect(filePosition(backwards, BAR, 2, 120)).toBe(1)
    expect(filePosition(backwards, BAR + BEAT, 2, 120)).toBeCloseTo(0.75, 12)
    // The offset skips from the end, which is where it starts playing.
    expect(
      filePosition(clip({ reverse: true, offset: BEAT }), BAR, 2, 120)
    ).toBeCloseTo(0.75, 12)
  })

  it("moves with the tempo: the same tick is another place in the file", () => {
    expect(filePosition(clip(), BAR + BEAT, 2, 60)).toBeCloseTo(0.5, 12)
    expect(filePosition(clip(), BAR + BEAT, 2, 240)).toBeCloseTo(0.125, 12)
  })

  it("has no place in a file with no length", () => {
    expect(filePosition(clip(), BAR, 0, 120)).toBeLessThan(0)
  })

  it("gives the two ends of a stretch of the timeline", () => {
    expect(fileWindow(clip(), BAR, BAR + 2 * BEAT, 2, 120)).toEqual({
      from: 0,
      to: 0.5,
    })
    const reversed = fileWindow(
      clip({ reverse: true }),
      BAR,
      BAR + 2 * BEAT,
      2,
      120
    )
    expect(reversed.from).toBe(1)
    expect(reversed.to).toBeCloseTo(0.5, 12)
  })
})

describe("fades", () => {
  it("are equal-power: the two halves of a crossfade add up to full power", () => {
    expect(fadeInGain(0)).toBe(0)
    expect(fadeInGain(1)).toBeCloseTo(1, 12)
    expect(fadeOutGain(0)).toBe(1)
    expect(fadeOutGain(1)).toBeCloseTo(0, 12)
    for (const part of [0.1, 0.25, 0.5, 0.8]) {
      expect(fadeInGain(part) ** 2 + fadeOutGain(part) ** 2).toBeCloseTo(1, 12)
    }
    expect(fadeInGain(0.5)).toBeCloseTo(Math.SQRT1_2, 12)
  })

  it("shape the level over the ticks at the clip's two ends", () => {
    const faded = { start: BAR, length: BAR, fadeIn: BEAT, fadeOut: 2 * BEAT }
    expect(fadeGainAt(faded, BAR)).toBe(0)
    expect(fadeGainAt(faded, BAR + BEAT / 2)).toBeCloseTo(Math.SQRT1_2, 12)
    expect(fadeGainAt(faded, BAR + BEAT)).toBe(1)
    expect(fadeGainAt(faded, 2 * BAR - BEAT)).toBeCloseTo(Math.SQRT1_2, 12)
    expect(fadeGainAt(faded, 2 * BAR)).toBeCloseTo(0, 12)
    // Outside the clip nothing sounds.
    expect(fadeGainAt(faded, BAR - 1)).toBe(0)
    expect(fadeGainAt(faded, 2 * BAR + 1)).toBe(0)
  })

  it("leave a clip with no fades at full level", () => {
    const plain = { start: 0, length: BAR, fadeIn: 0, fadeOut: 0 }
    expect(fadeGainAt(plain, 0)).toBe(1)
    expect(fadeGainAt(plain, BAR)).toBe(1)
  })

  it("stay inside the clip and on the grid", () => {
    expect(clampFade(500, BAR, 240)).toBe(480)
    expect(clampFade(500, BAR, 0)).toBe(500)
    expect(clampFade(-80, BAR, 240)).toBe(0)
    expect(clampFade(9 * BAR, BAR, 240)).toBe(BAR)
  })

  it("follow the pointer from the edge they belong to", () => {
    const span = { start: BAR, length: BAR }
    expect(fadeFromPointer("in", span, BAR + 700, 240)).toBe(720)
    expect(fadeFromPointer("in", span, BAR + 700, 0)).toBe(700)
    expect(fadeFromPointer("in", span, 0, 240)).toBe(0)
    expect(fadeFromPointer("out", span, 2 * BAR - 700, 240)).toBe(720)
    expect(fadeFromPointer("out", span, 9 * BAR, 240)).toBe(0)
    expect(fadeFromPointer("out", span, 0, 240)).toBe(BAR)
  })

  it("are described in beats and in time", () => {
    expect(describeFade(0, 120)).toBe("No fade")
    expect(describeFade(BEAT, 120)).toBe("1 beat, 500 ms")
    expect(describeFade(BEAT / 2, 120)).toBe("0.5 beats, 250 ms")
    expect(describeFade(BAR, 60)).toBe("4 beats, 4.00 s")
    expect(describeFade(100, 120)).toBe("0.10 beats, 52 ms")
  })
})

describe("a gain drag", () => {
  it("moves a quarter of a decibel a pixel, and a tenth of that when fine", () => {
    expect(gainFromDrag(1, 24, false)).toBeCloseTo(10 ** (6 / 20), 9)
    expect(gainFromDrag(1, -24, false)).toBeCloseTo(10 ** (-6 / 20), 9)
    expect(gainFromDrag(1, 40, true)).toBeCloseTo(10 ** (1 / 20), 9)
  })

  it("stops at +6 dB and falls to silence below the floor", () => {
    expect(gainFromDrag(1, 500, false)).toBe(2)
    expect(gainFromDrag(1, -239, false)).toBeGreaterThan(0)
    expect(gainFromDrag(1, -400, false)).toBe(0)
    // Up from silence starts at the floor, not at minus infinity.
    expect(gainFromDrag(0, 40, false)).toBeCloseTo(10 ** (-50 / 20), 9)
  })
})
