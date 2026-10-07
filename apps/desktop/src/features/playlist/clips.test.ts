import { describe, expect, it } from "vitest"

import type { Automation, ClipContent, Pattern, Project } from "@/bindings"
import type { Viewport } from "@/lib/canvas"

import { brushClip, brushName, brushTicks } from "./brush"
import { clipBox, contentArea, hasBand, scaleBox } from "./clip-box"
import { contentKey } from "./edit"
import { cursorFor, intentFor, type Press } from "./intents"
import {
  clipPattern,
  colorSignature,
  contentColor,
  contentName,
  lookupsOf,
  NO_LOOKUPS,
  ORPHAN_COLOR,
} from "./look"
import { viewportShowing } from "./reveal"
import type { ResolvedBrush } from "./selectors"

/* The small pure parts that every kind of clip goes through. */

const BAR = 3840

const pattern: Pattern = {
  id: 2,
  name: "Verse",
  color: 0x112233,
  lengthSteps: 32,
  lanes: [],
}
const automation: Automation = {
  id: 7,
  name: "Kick volume",
  color: 0x445566,
  target: { type: "tempo" },
  points: [
    { tick: 0, value: 0, curve: 0, hold: false },
    { tick: 3 * BAR, value: 1, curve: 0, hold: false },
  ],
}
const audio: ClipContent = {
  type: "audio",
  sample: 4,
  mixerTrack: 9,
  gain: 1,
  pan: 0,
  fadeIn: 0,
  fadeOut: 0,
  reverse: false,
  pitch: 0,
}

const project = {
  patterns: [pattern],
  samples: [
    { id: 4, name: "Vocal", path: { kind: "external", path: "/v.wav" } },
  ],
  mixer: {
    tracks: [
      {
        id: 9,
        name: "Vox",
        color: 0x778899,
        volume: 1,
        pan: 0,
        muted: false,
        solo: false,
        output: 0,
        sends: [],
        effects: [],
      },
    ],
  },
  automations: [automation],
} satisfies Pick<Project, "patterns" | "samples" | "mixer" | "automations">

describe("what a clip looks like", () => {
  const lookups = lookupsOf(project)
  const asPattern: ClipContent = { type: "pattern", pattern: 2 }
  const asCurve: ClipContent = { type: "automation", automation: 7 }

  it("takes its color from its pattern, its automation or its mixer track", () => {
    expect(contentColor(asPattern, lookups)).toBe(0x112233)
    expect(contentColor(asCurve, lookups)).toBe(0x445566)
    expect(contentColor(audio, lookups)).toBe(0x778899)
  })

  it("is named after its pattern, its automation or its sample", () => {
    expect(contentName(asPattern, lookups)).toBe("Verse")
    expect(contentName(asCurve, lookups)).toBe("Kick volume")
    expect(contentName(audio, lookups)).toBe("Vocal")
  })

  it("has a stand-in color and no name when what it plays is gone", () => {
    for (const content of [asPattern, asCurve, audio]) {
      expect(contentColor(content, NO_LOOKUPS)).toBe(ORPHAN_COLOR)
      expect(contentName(content, NO_LOOKUPS)).toBe("")
    }
  })

  it("has a pattern only when it is a pattern clip", () => {
    expect(clipPattern({ content: asPattern }, lookups)).toBe(pattern)
    expect(clipPattern({ content: audio }, lookups)).toBeUndefined()
  })

  it("tells content apart by what it plays, not by how", () => {
    expect(contentKey(asPattern)).not.toBe(contentKey(asCurve))
    expect(contentKey(audio)).toBe(contentKey({ ...audio, gain: 0.2 }))
    expect(contentKey(audio)).not.toBe(contentKey({ ...audio, sample: 5 }))
  })

  it("changes its color signature for a new color and not for a fader move", () => {
    const before = colorSignature(lookups)
    const moved = lookupsOf({
      ...project,
      mixer: { tracks: [{ ...project.mixer.tracks[0], volume: 0.3 }] },
    })
    expect(colorSignature(moved)).toBe(before)
    const recolored = lookupsOf({
      ...project,
      mixer: { tracks: [{ ...project.mixer.tracks[0], color: 0xff0000 }] },
    })
    expect(colorSignature(recolored)).not.toBe(before)
  })
})

describe("the brush", () => {
  const context = { barTicks: BAR, tempoBpm: 120, durationSecs: null }
  const patternBrush: ResolvedBrush = { type: "pattern", pattern }
  const audioBrush: ResolvedBrush = {
    type: "audio",
    sample: project.samples[0],
  }
  const curveBrush: ResolvedBrush = { type: "automation", automation }

  it("is one pass of a pattern long", () => {
    expect(brushTicks(patternBrush, context)).toBe(2 * BAR)
  })

  it("is as long as a sample lasts, or a bar until that is known", () => {
    expect(brushTicks(audioBrush, context)).toBe(BAR)
    expect(brushTicks(audioBrush, { ...context, durationSecs: 3 })).toBe(5760)
    expect(
      brushTicks(audioBrush, { ...context, durationSecs: 3, tempoBpm: 60 })
    ).toBe(2880)
  })

  it("is as long as an automation's curve, and at least a bar", () => {
    expect(brushTicks(curveBrush, context)).toBe(3 * BAR)
    const short: ResolvedBrush = {
      type: "automation",
      automation: { ...automation, points: automation.points.slice(0, 1) },
    }
    expect(brushTicks(short, context)).toBe(BAR)
  })

  it("makes a clip of what it places", () => {
    expect(brushClip(patternBrush, BAR, 2, context)).toEqual({
      row: 2,
      start: BAR,
      length: 2 * BAR,
      offset: 0,
      muted: false,
      content: { type: "pattern", pattern: 2 },
    })
    expect(brushClip(curveBrush, 0, 0, context).content).toEqual({
      type: "automation",
      automation: 7,
    })
    // An audio clip plays into the track its sample already uses.
    expect(
      brushClip(audioBrush, 0, 0, { ...context, mixerTrack: 9 }).content
    ).toEqual(audio)
    expect(brushClip(audioBrush, 0, 0, context).content).toMatchObject({
      mixerTrack: 0,
    })
  })

  it("has a name for the status bar", () => {
    expect(brushName(patternBrush)).toBe("Verse")
    expect(brushName(audioBrush)).toBe("Vocal")
    expect(brushName(curveBrush)).toBe("Kick volume")
    expect(brushName(null)).toBe("a pattern")
  })
})

describe("where a clip is on screen", () => {
  const viewport: Viewport = {
    width: 800,
    height: 400,
    dpr: 1,
    scrollTick: BAR,
    scrollRow: 1,
    pxPerTick: 0.025,
    rowHeight: 40,
  }

  it("is a box in pixels, less the line above its row", () => {
    expect(clipBox(viewport, { start: 2 * BAR, length: BAR }, 3)).toEqual({
      left: 96,
      right: 192,
      top: 81,
      bottom: 120,
    })
  })

  it("draws its content under a title bar when the row is tall enough", () => {
    const tall = clipBox(viewport, { start: BAR, length: BAR }, 1)
    expect(hasBand(tall)).toBe(true)
    expect(contentArea(tall)).toEqual({
      left: 1,
      right: 95,
      top: 18,
      bottom: 36,
    })
    const short = clipBox(
      { ...viewport, rowHeight: 20 },
      { start: BAR, length: BAR },
      1
    )
    expect(hasBand(short)).toBe(false)
    expect(contentArea(short)).toEqual({
      left: 1,
      right: 95,
      top: 5,
      bottom: 16,
    })
    expect(scaleBox(tall, 2)).toEqual({
      left: 0,
      right: 192,
      top: 2,
      bottom: 80,
    })
  })
})

describe("scrolling to a clip", () => {
  const viewport: Viewport = {
    width: 960,
    height: 400,
    dpr: 1,
    scrollTick: 0,
    scrollRow: 0,
    pxPerTick: 0.025,
    rowHeight: 40,
  }

  it("leaves the view alone when the clip's start and row are in it", () => {
    expect(viewportShowing(viewport, { start: BAR, length: BAR }, 3)).toEqual(
      viewport
    )
  })

  it("scrolls sideways to a clip that starts out of view", () => {
    const far = viewportShowing(viewport, { start: 40 * BAR, length: BAR }, 3)
    expect(far.scrollTick).toBe(40 * BAR - BAR / 2)
    expect(far.scrollRow).toBe(0)
    const behind = viewportShowing(
      { ...viewport, scrollTick: 20 * BAR },
      { start: 0, length: BAR },
      0
    )
    expect(behind.scrollTick).toBe(0)
  })

  it("scrolls to a row that is out of view", () => {
    const below = viewportShowing(viewport, { start: 0, length: BAR }, 25)
    expect(below.scrollRow).toBe(21)
    expect(below.scrollTick).toBe(0)
  })
})

describe("what a press on the inside of a clip starts", () => {
  const body = { id: 7, part: "body" } as const
  const edge = { id: 7, part: "start-edge" } as const
  const press = (overrides: Partial<Press>): Press => ({
    tool: "draw",
    button: 0,
    mod: false,
    shift: false,
    hit: body,
    ...overrides,
  })

  it("drags the handles of an audio clip", () => {
    expect(intentFor(press({ inner: { kind: "gain" } }))).toEqual({
      kind: "gain",
      id: 7,
    })
    expect(
      intentFor(press({ hit: edge, inner: { kind: "fade", edge: "in" } }))
    ).toEqual({ kind: "fade", id: 7, edge: "in" })
  })

  it("edits a curve: a point, a bend, or a new point in the open area", () => {
    expect(intentFor(press({ inner: { kind: "point", index: 2 } }))).toEqual({
      kind: "point",
      id: 7,
      index: 2,
    })
    expect(intentFor(press({ inner: { kind: "bend", index: 1 } }))).toEqual({
      kind: "bend",
      id: 7,
      index: 1,
    })
    expect(intentFor(press({ inner: { kind: "curve" } }))).toEqual({
      kind: "add-point",
      id: 7,
    })
  })

  it("takes a point on the clip's edge, and leaves the rest of the edge to resizing", () => {
    expect(
      intentFor(press({ hit: edge, inner: { kind: "point", index: 0 } }))
    ).toEqual({ kind: "point", id: 7, index: 0 })
    expect(intentFor(press({ hit: edge, inner: { kind: "curve" } }))).toEqual({
      kind: "trim-start",
      id: 7,
    })
    expect(
      intentFor(press({ hit: edge, inner: { kind: "bend", index: 0 } }))
    ).toEqual({ kind: "trim-start", id: 7 })
  })

  it("moves or copies the clip from anywhere on it with Ctrl", () => {
    expect(
      intentFor(press({ mod: true, inner: { kind: "point", index: 0 } }))
    ).toEqual({ kind: "move", id: 7, additive: false })
  })

  it("deletes a point with the right button, or opens its menu in the Select tool", () => {
    const point = { kind: "point", index: 3 } as const
    expect(intentFor(press({ button: 2, inner: point }))).toEqual({
      kind: "delete-point",
      id: 7,
      index: 3,
    })
    expect(
      intentFor(press({ tool: "select", button: 2, inner: point }))
    ).toEqual({ kind: "point-menu", id: 7, index: 3 })
    // A right-click that misses the points spares the clip.
    expect(intentFor(press({ button: 2, inner: { kind: "curve" } }))).toEqual({
      kind: "none",
    })
    expect(
      intentFor(press({ tool: "select", button: 2, inner: { kind: "curve" } }))
    ).toEqual({ kind: "menu", id: 7 })
    // On a handle of an audio clip it deletes the clip, as anywhere on it.
    expect(intentFor(press({ button: 2, inner: { kind: "gain" } }))).toEqual({
      kind: "erase",
    })
  })

  it("has nothing of its own in the Erase and Mute tools", () => {
    const point = { kind: "point", index: 0 } as const
    expect(intentFor(press({ tool: "erase", inner: point }))).toEqual({
      kind: "erase",
    })
    expect(intentFor(press({ tool: "mute", inner: point }))).toEqual({
      kind: "mute",
    })
  })

  it("shows by the cursor what a press would do", () => {
    expect(cursorFor("draw", "body", { kind: "gain" })).toBe("ns-resize")
    expect(cursorFor("draw", "body", { kind: "fade", edge: "out" })).toBe(
      "ew-resize"
    )
    expect(cursorFor("select", "start-edge", { kind: "point", index: 0 })).toBe(
      "move"
    )
    expect(cursorFor("draw", "body", { kind: "bend", index: 0 })).toBe(
      "ns-resize"
    )
    expect(cursorFor("draw", "body", { kind: "curve" })).toBe("crosshair")
    expect(cursorFor("draw", "end-edge", { kind: "curve" })).toBe("ew-resize")
    expect(cursorFor("erase", "body", { kind: "curve" })).toBe("not-allowed")
  })
})
