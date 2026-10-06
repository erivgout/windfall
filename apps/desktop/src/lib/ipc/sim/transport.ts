import type {
  Channel,
  Pattern,
  Project,
  RealtimeFrame,
  TransportPatch,
  TransportState,
} from "@/bindings"
import { PPQ, TICKS_PER_STEP } from "@/lib/units"

type Onset = { channel: number; velocity: number }

/** How fast a simulated meter falls back, as a time constant in seconds. */
const METER_FALL_SECS = 0.14

function patternOnsets(pattern: Pattern, from: number, to: number): Onset[] {
  const onsets: Onset[] = []
  for (const lane of pattern.lanes) {
    for (const note of lane.notes) {
      if (note.start >= from && note.start < to) {
        onsets.push({ channel: lane.channel, velocity: note.velocity })
      }
    }
  }
  return onsets
}

function songLength(project: Project): number {
  const end = project.playlist.clips.reduce(
    (latest, clip) => Math.max(latest, clip.start + clip.length),
    0
  )
  const { numerator, denominator } = project.settings.timeSignature
  const bar = (numerator * PPQ * 4) / denominator
  return end > 0 ? end : bar * 4
}

function songOnsets(project: Project, from: number, to: number): Onset[] {
  const onsets: Onset[] = []
  for (const clip of project.playlist.clips) {
    const track = project.playlist.tracks.find((item) => item.id === clip.track)
    if (clip.muted || track?.muted) continue
    if (clip.start >= to || clip.start + clip.length <= from) continue
    const pattern = project.patterns.find(
      (item) => item.id === clip.content.pattern
    )
    if (!pattern) continue
    const length = pattern.lengthSteps * TICKS_PER_STEP
    for (const lane of pattern.lanes) {
      for (const note of lane.notes) {
        if (note.start >= length) continue
        // The clip loops its pattern, so a note sounds once per pass.
        let at = clip.start + note.start - (clip.offset % length)
        if (at < clip.start) at += length
        for (; at < clip.start + clip.length; at += length) {
          if (at >= from && at < to) {
            onsets.push({ channel: lane.channel, velocity: note.velocity })
          }
        }
      }
    }
  }
  return onsets
}

function audibleChannel(channel: Channel, anySolo: boolean): boolean {
  if (channel.muted) return false
  return anySolo ? channel.solo : true
}

/**
 * A stand-in for the engine's transport and meters: a playhead that follows
 * the tempo, and meters that jump when a note starts and then fall.
 */
export class TransportSim {
  state: TransportState
  tick = 0
  private levels = new Map<number, number>()

  constructor(pattern: number) {
    this.state = { playing: false, mode: "pattern", pattern, loopSong: true }
  }

  play() {
    this.state = { ...this.state, playing: true }
  }

  stop() {
    this.state = { ...this.state, playing: false }
    this.tick = 0
  }

  seek(tick: number) {
    this.tick = Math.max(0, tick)
  }

  set(patch: TransportPatch) {
    const modeChanged =
      patch.mode !== undefined && patch.mode !== this.state.mode
    const patternChanged =
      patch.pattern !== undefined && patch.pattern !== this.state.pattern
    this.state = {
      ...this.state,
      mode: patch.mode ?? this.state.mode,
      pattern: patch.pattern ?? this.state.pattern,
      loopSong: patch.loopSong ?? this.state.loopSong,
    }
    if (modeChanged || (patternChanged && this.state.mode === "pattern")) {
      this.tick = 0
    }
  }

  /** Moves time forward and reports what the engine would. */
  advance(project: Project, seconds: number): RealtimeFrame {
    const onsets = this.state.playing ? this.step(project, seconds) : []
    const fall = Math.exp(-seconds / METER_FALL_SECS)
    for (const [id, level] of this.levels) {
      this.levels.set(id, level < 0.001 ? 0 : level * fall)
    }

    const anySolo = project.channels.some((channel) => channel.solo)
    for (const onset of onsets) {
      const channel = project.channels.find((item) => item.id === onset.channel)
      if (!channel || !audibleChannel(channel, anySolo)) continue
      const hit = onset.velocity * channel.volume * channel.source.gain
      const current = this.levels.get(channel.mixerTrack) ?? 0
      this.levels.set(channel.mixerTrack, Math.max(current, hit))
    }

    const meters = this.meters(project)
    const voices = [...this.levels.values()].filter((level) => level > 0.05)
    return {
      playing: this.state.playing,
      tick: this.tick,
      meters,
      cpu: 0.03 + voices.length * 0.012 + Math.random() * 0.006,
      xruns: 0,
      voices: voices.length,
    }
  }

  private step(project: Project, seconds: number): Onset[] {
    const from = this.tick
    const to = from + (seconds * project.settings.tempoBpm * PPQ) / 60

    if (this.state.mode === "song") {
      const length = songLength(project)
      if (to < length) {
        this.tick = to
        return songOnsets(project, from, to)
      }
      const onsets = songOnsets(project, from, length)
      if (!this.state.loopSong) {
        this.stop()
        return onsets
      }
      this.tick = to - length
      return [...onsets, ...songOnsets(project, 0, this.tick)]
    }

    const pattern = project.patterns.find(
      (item) => item.id === this.state.pattern
    )
    if (!pattern) return []
    const length = pattern.lengthSteps * TICKS_PER_STEP
    if (to < length) {
      this.tick = to
      return patternOnsets(pattern, from, to)
    }
    this.tick = (to - length) % length
    return [
      ...patternOnsets(pattern, from, length),
      ...patternOnsets(pattern, 0, this.tick),
    ]
  }

  /** Two values per mixer track, in mixer order, the master first. */
  private meters(project: Project): number[] {
    const tracks = project.mixer.tracks
    const anySolo = tracks.some((track) => track.solo)
    const outputs = new Map<number, number>()

    const outputOf = (id: number, depth: number): number => {
      const cached = outputs.get(id)
      if (cached !== undefined) return cached
      const track = tracks.find((item) => item.id === id)
      if (!track || depth > tracks.length) return 0
      let input = this.levels.get(id) ?? 0
      for (const other of tracks) {
        if (other.output === id) input += outputOf(other.id, depth + 1) * 0.7
      }
      const silent = track.muted || (anySolo && !track.solo && id !== 0)
      const level = silent ? 0 : Math.min(1.4, input * track.volume)
      outputs.set(id, level)
      return level
    }

    return tracks.flatMap((track) => {
      const level = outputOf(track.id, 0)
      const left = level * Math.min(1, 1 - track.pan)
      const right = level * Math.min(1, 1 + track.pan)
      return [left, right * (0.94 + Math.random() * 0.06)]
    })
  }
}
