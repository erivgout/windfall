/** Original synthetic fixture dedicated to CC0-1.0; no Image-Line bytes. */
export function flpFixture() {
  const events: number[] = []
  const word = (id: number, value: number) =>
    events.push(id, value & 255, value >> 8)
  const data = (id: number, bytes: number[]) => {
    events.push(id, bytes.length, ...bytes)
  }
  const text = (id: number, value: string) =>
    data(
      id,
      [...value, "\0"].flatMap((c) => [c.charCodeAt(0), 0])
    )
  data(199, [...new TextEncoder().encode("20.8.4.2576"), 0])
  word(64, 0)
  events.push(21, 2)
  text(201, "Unknown synth")
  data(213, [0, 255, 17, 90])
  word(64, 1)
  events.push(21, 0)
  text(196, "missing.wav")
  word(65, 1)
  const note = new Uint8Array(24)
  const view = new DataView(note.buffer)
  view.setUint32(8, 96, true)
  view.setUint16(12, 60, true)
  note[16] = 120
  note[20] = 64
  note[21] = 100
  data(224, Array.from(note))
  const bytes = new Uint8Array(22 + events.length)
  const header = new DataView(bytes.buffer)
  bytes.set(new TextEncoder().encode("FLhd"))
  header.setUint32(4, 6, true)
  header.setUint16(10, 2, true)
  header.setUint16(12, 96, true)
  bytes.set(new TextEncoder().encode("FLdt"), 14)
  header.setUint32(18, events.length, true)
  bytes.set(events, 22)
  return bytes
}
