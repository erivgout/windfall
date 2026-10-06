export type DemoChannel = {
  id: number
  name: string
  /** Step color as a CSS color. Unset uses the theme's step color. */
  color?: string
  gain: number
  pan: number
  muted: boolean
  solo: boolean
  /** How fast a hit dies away, per second. */
  decay: number
  steps: boolean[]
}

const pattern = (text: string) => [...text].map((mark) => mark === "x")

const TEAL = "oklch(0.74 0.13 195)"
const AMBER = "oklch(0.8 0.15 75)"

export const DEMO_CHANNELS: DemoChannel[] = [
  { name: "Kick", steps: "x...x...x...x..x", gain: 1.2, pan: 0, decay: 9 },
  { name: "Clap", steps: "....x.......x...", gain: 0.8, pan: 0.1, decay: 12 },
  {
    name: "Closed hat",
    steps: "x.x.x.x.x.x.x.xx",
    gain: 0.5,
    pan: -0.3,
    decay: 26,
  },
  {
    name: "Open hat",
    steps: "..x...x...x...x.",
    gain: 0.45,
    pan: 0.3,
    decay: 7,
  },
  {
    name: "Snare",
    steps: "....x..x.x..x...",
    gain: 0.9,
    pan: 0,
    decay: 11,
    color: AMBER,
  },
  {
    name: "Rim",
    steps: "...x..x....x..x.",
    gain: 0.4,
    pan: -0.5,
    decay: 20,
    color: AMBER,
  },
  {
    name: "Low tom",
    steps: "..........x..x..",
    gain: 0.7,
    pan: 0.45,
    decay: 8,
    color: TEAL,
  },
  {
    name: "Shaker",
    steps: "xxxxxxxxxxxxxxxx",
    gain: 0.25,
    pan: 0.6,
    decay: 30,
    color: TEAL,
  },
].map((channel, id) => ({
  ...channel,
  id,
  muted: false,
  solo: false,
  steps: pattern(channel.steps),
}))

/** A drum-like hit as min and max pairs, the format `Waveform` draws. */
export function makeDemoPeaks(buckets = 600): Float32Array {
  const peaks = new Float32Array(buckets * 2)
  let seed = 7
  const random = () => {
    seed = (seed * 16807) % 2147483647
    return seed / 2147483647
  }
  for (let bucket = 0; bucket < buckets; bucket += 1) {
    const time = bucket / buckets
    const body = Math.exp(-time * 5.5)
    const click = Math.exp(-time * 60)
    const wobble = Math.sin(time * 90 * (1 - time * 0.6))
    const noise = random() * 0.35 * Math.exp(-time * 9)
    const high = Math.min(
      1,
      body * (0.55 + 0.4 * Math.abs(wobble)) + click * 0.4 + noise
    )
    const low = -Math.min(
      1,
      body * (0.5 + 0.42 * Math.abs(Math.cos(time * 77))) + click * 0.35 + noise
    )
    peaks[bucket * 2] = low
    peaks[bucket * 2 + 1] = high
  }
  return peaks
}

/** The balance law the engine uses: the far side falls, the near side stays. */
export function panGains(pan: number): [left: number, right: number] {
  return [pan > 0 ? 1 - pan : 1, pan < 0 ? 1 + pan : 1]
}
