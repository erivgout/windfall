import type { ChannelId, Note, Pattern } from "@/bindings"
import { ticksPerBar } from "@/lib/time"
import { PPQ } from "@/lib/units"

type Chord = { start: number; end: number; notes: Note[] }
type Voice = { end: number; chords: Chord[] }

function escapeXml(text: string): string {
  return text.replace(/[&<>"']/g, (character) => {
    switch (character) {
      case "&":
        return "&amp;"
      case "<":
        return "&lt;"
      case ">":
        return "&gt;"
      case '"':
        return "&quot;"
      default:
        return "&apos;"
    }
  })
}

function pitch(key: number): string {
  const steps = ["C", "C", "D", "D", "E", "F", "F", "G", "G", "A", "A", "B"]
  const semitone = key % 12
  const alter = [1, 3, 6, 8, 10].includes(semitone) ? "<alter>1</alter>" : ""
  return `<pitch><step>${steps[semitone]}</step>${alter}<octave>${Math.floor(key / 12) - 1}</octave></pitch>`
}

function rest(duration: number, voice: number, wholeMeasure = false): string {
  return `<note><rest${wholeMeasure ? ' measure="yes"' : ""}/><duration>${duration}</duration><voice>${voice}</voice></note>`
}

/** One channel, in project ticks. No project edits or engraving are involved. */
export function buildChannelMusicXml(
  pattern: Pattern,
  channel: ChannelId,
  partName = "Piano"
): string {
  const signature = pattern.timeline?.meters.findLast(
    (meter) => meter.tick === 0
  )?.signature ??
    pattern.timeSignature ?? { numerator: 4, denominator: 4 }
  const barTicks = ticksPerBar(signature)
  const notes = [
    ...(pattern.lanes.find((lane) => lane.channel === channel)?.notes ?? []),
  ].sort(
    (a, b) =>
      a.start - b.start || b.length - a.length || a.key - b.key || a.id - b.id
  )
  const chords: Chord[] = []
  for (const note of notes) {
    const previous = chords.at(-1)
    if (previous?.start === note.start) previous.notes.push(note)
    else
      chords.push({
        start: note.start,
        end: note.start + note.length,
        notes: [note],
      })
  }

  // Keep same-start notes together; independent overlaps need another voice.
  // The longest chord tone comes first, since only it advances MusicXML time.
  const voices: Voice[] = []
  for (const chord of chords) {
    let voice = voices.find((candidate) => candidate.end <= chord.start)
    if (!voice) {
      voice = { end: 0, chords: [] }
      voices.push(voice)
    }
    voice.chords.push(chord)
    voice.end = chord.end
  }
  if (!voices.length) voices.push({ end: 0, chords: [] })
  const end = Math.max(barTicks, ...voices.map((voice) => voice.end))
  const xml = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<score-partwise version="4.0">',
    `<work><work-title>${escapeXml(pattern.name)}</work-title></work>`,
    "<identification><encoding><software>Windfall</software></encoding></identification>",
    `<part-list><score-part id="P1"><part-name>${escapeXml(partName)}</part-name></score-part></part-list>`,
    '<part id="P1">',
  ]

  for (let start = 0; start < end; start += barTicks) {
    const measureEnd = start + barTicks
    xml.push(`<measure number="${start / barTicks + 1}">`)
    if (start === 0) {
      xml.push(
        `<attributes><divisions>${PPQ}</divisions><time><beats>${signature.numerator}</beats><beat-type>${signature.denominator}</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>`
      )
    }
    voices.forEach((voice, index) => {
      const number = index + 1
      if (index) xml.push(`<backup><duration>${barTicks}</duration></backup>`)
      let cursor = start
      for (const chord of voice.chords) {
        if (chord.end <= start || chord.start >= measureEnd) continue
        const onset = Math.max(start, chord.start)
        if (onset > cursor) xml.push(rest(onset - cursor, number))
        const tones = chord.notes.filter(
          (note) => note.start + note.length > start
        )
        tones.forEach((note, tone) => {
          const noteEnd = note.start + note.length
          const duration = Math.min(noteEnd, measureEnd) - onset
          const ties = [
            ...(note.start < start ? ["stop"] : []),
            ...(noteEnd > measureEnd ? ["start"] : []),
          ]
          xml.push(
            `<note>${tone ? "<chord/>" : ""}${pitch(note.key)}<duration>${duration}</duration>${ties.map((type) => `<tie type="${type}"/>`).join("")}<voice>${number}</voice>${ties.length ? `<notations>${ties.map((type) => `<tied type="${type}"/>`).join("")}</notations>` : ""}</note>`
          )
        })
        cursor = Math.min(chord.end, measureEnd)
      }
      if (cursor < measureEnd)
        xml.push(rest(measureEnd - cursor, number, cursor === start))
    })
    xml.push("</measure>")
  }
  xml.push("</part>", "</score-partwise>")
  return xml.join("\n") + "\n"
}
