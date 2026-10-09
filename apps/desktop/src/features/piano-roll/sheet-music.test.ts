import { describe, expect, it } from "vitest"

import type { Note, Pattern } from "@/bindings"
import { PPQ } from "@/lib/units"

import { buildChannelMusicXml } from "./sheet-music"

function note(key = 60, start = 0, length = PPQ, id = 1): Note {
  return { id, key, start, length, velocity: 0.8, pan: 0 }
}

function pattern(notes: Note[]): Pattern {
  return {
    id: 1,
    name: "Idea & variation",
    color: 0,
    lengthSteps: 16,
    lanes: [
      { channel: 1, notes },
      { channel: 2, notes: [note(72)] },
    ],
  }
}

function score(source: Pattern): XMLDocument {
  const document = new DOMParser().parseFromString(
    buildChannelMusicXml(source, 1, "Piano <one>"),
    "application/xml"
  )
  expect(document.querySelector("parsererror")).toBeNull()
  expect(document.documentElement.tagName).toBe("score-partwise")
  expect(document.querySelectorAll("part")).toHaveLength(1)
  expect(document.querySelector("part")?.id).toBe("P1")
  return document
}

describe("piano-roll MusicXML builder", () => {
  it("exports C4 pitch and tick duration for only the requested channel", () => {
    const xml = score(pattern([note()]))
    const pitched = xml.querySelectorAll("note:has(pitch)")
    expect(pitched).toHaveLength(1)
    expect(pitched[0].querySelector("step")?.textContent).toBe("C")
    expect(pitched[0].querySelector("octave")?.textContent).toBe("4")
    expect(pitched[0].querySelector("duration")?.textContent).toBe(String(PPQ))
    expect(xml.querySelector("divisions")?.textContent).toBe(String(PPQ))
    expect(xml.querySelector("beats")?.textContent).toBe("4")
    expect(xml.querySelector("beat-type")?.textContent).toBe("4")
    expect(xml.querySelector("work-title")?.textContent).toBe(
      "Idea & variation"
    )
    expect(xml.querySelector("part-name")?.textContent).toBe("Piano <one>")
  })

  it("makes notes at the same start a chord, with the longest tone first", () => {
    const xml = score(pattern([note(60, 0, 240), note(64, 0, PPQ, 2)]))
    const notes = xml.querySelectorAll("note:has(pitch)")
    expect(notes).toHaveLength(2)
    expect(notes[0].querySelector("chord")).toBeNull()
    expect(notes[0].querySelector("duration")?.textContent).toBe(String(PPQ))
    expect(notes[1].querySelector("chord")).not.toBeNull()
    expect(notes[1].querySelector("duration")?.textContent).toBe("240")
    expect(notes[0].querySelector("voice")?.textContent).toBe(
      notes[1].querySelector("voice")?.textContent
    )
  })

  it("turns leading and interior gaps into rests", () => {
    const xml = score(pattern([note(60, 240, 240), note(62, 960, 240, 2)]))
    const notes = xml.querySelectorAll("note")
    expect(notes[0].querySelector("rest")).not.toBeNull()
    expect(notes[0].querySelector("duration")?.textContent).toBe("240")
    expect(notes[2].querySelector("rest")).not.toBeNull()
    expect(notes[2].querySelector("duration")?.textContent).toBe("480")
  })

  it("exports one whole-measure rest for an empty channel, even with other lanes", () => {
    const source = pattern([])
    source.lanes = source.lanes.filter((lane) => lane.channel !== 1)
    const xml = score(source)
    expect(xml.querySelectorAll("measure")).toHaveLength(1)
    expect(xml.querySelectorAll("note")).toHaveLength(1)
    expect(xml.querySelector('rest[measure="yes"]')).not.toBeNull()
    expect(xml.querySelector("duration")?.textContent).toBe(String(4 * PPQ))
    expect(xml.querySelector("pitch")).toBeNull()
  })

  it("uses the pattern meter at tick zero and keeps it for the score", () => {
    const source = pattern([])
    source.timeSignature = { numerator: 3, denominator: 4 }
    source.timeline = {
      markers: [],
      meters: [
        { id: 3, tick: 0, signature: { numerator: 6, denominator: 8 } },
        { id: 4, tick: PPQ, signature: { numerator: 2, denominator: 4 } },
      ],
    }
    const xml = score(source)
    expect(xml.querySelector("beats")?.textContent).toBe("6")
    expect(xml.querySelector("beat-type")?.textContent).toBe("8")
    expect(xml.querySelector("duration")?.textContent).toBe(String(3 * PPQ))
    delete source.timeline
    expect(score(source).querySelector("beats")?.textContent).toBe("3")
  })

  it("splits sustained notes at bar lines with sounding and notation ties", () => {
    const xml = score(pattern([note(61, 3 * PPQ, 2 * PPQ)]))
    const notes = xml.querySelectorAll("note:has(pitch)")
    expect(xml.querySelectorAll("measure")).toHaveLength(2)
    expect(
      [...notes].map((item) => item.querySelector("duration")?.textContent)
    ).toEqual([String(PPQ), String(PPQ)])
    expect(notes[0].querySelector('tie[type="start"]')).not.toBeNull()
    expect(notes[1].querySelector('tie[type="stop"]')).not.toBeNull()
    expect(notes[0].querySelector('tied[type="start"]')).not.toBeNull()
    expect(notes[1].querySelector('tied[type="stop"]')).not.toBeNull()
    expect(notes[0].querySelector("alter")?.textContent).toBe("1")
  })

  it("keeps different-start overlapping notes in independent voices", () => {
    const xml = score(pattern([note(60, 0, 2 * PPQ), note(64, PPQ, PPQ, 2)]))
    const notes = xml.querySelectorAll("note:has(pitch)")
    expect(
      [...notes].map((item) => item.querySelector("voice")?.textContent)
    ).toEqual(["1", "2"])
    expect(xml.querySelectorAll("chord")).toHaveLength(0)
    expect(xml.querySelector("backup duration")?.textContent).toBe(
      String(4 * PPQ)
    )
    const secondVoice = xml.querySelector("backup + note")
    expect(secondVoice?.querySelector("rest")).not.toBeNull()
    expect(secondVoice?.querySelector("duration")?.textContent).toBe(
      String(PPQ)
    )
  })
})
