import type { EffectSlot, GainReduction, Project, TrackId } from "@/bindings"
import { gainToDb, MASTER_TRACK } from "@/lib/units"

/** Samples the synth's oscillators put their output out late by. */
const INSTRUMENT_LATENCY_FRAMES = 12

function slotLatency(slot: EffectSlot, sampleRate: number): number {
  const params = slot.params
  if (params.type === "limiter") {
    return Math.round((params.lookaheadMs * sampleRate) / 1000)
  }
  if (params.type === "stereoMatrix") {
    return Math.min(
      Math.round((params.leftDelayMs * sampleRate) / 1000),
      Math.round((params.rightDelayMs * sampleRate) / 1000)
    )
  }
  if (params.type === "distortion") return 32
  return 0
}

/** Declared built-in routing latency for the UI simulator; no browser DSP. */
export function simulatedLatencyFrames(project: Project, sampleRate: number): number {
  const tracks = project.mixer.tracks
  const memo = new Map<TrackId, number>()
  const visiting = new Set<TrackId>()
  const output = (id: TrackId): number => {
    const saved = memo.get(id)
    if (saved !== undefined) return saved
    const track = tracks.find((item) => item.id === id)
    if (!track || visiting.has(id)) return 0
    visiting.add(id)
    let arrival = 0
    for (const source of tracks) {
      if (source.output === id && !source.externalOutput?.exclusive || source.sends.some((send) => send.target === id)) arrival = Math.max(arrival, output(source.id))
    }
    for (const channel of project.channels) {
      if (channel.mixerTrack === id && channel.source.type === "instrument" && !project.plugins?.some((plugin) => plugin.target.type === "instrument" && plugin.target.channel === channel.id)) arrival = Math.max(arrival, INSTRUMENT_LATENCY_FRAMES)
    }
    const own = track.effects.reduce((sum, slot) => sum + (project.plugins?.some((plugin) => plugin.target.type === "effect" && plugin.target.effect === slot.id) ? 0 : slotLatency(slot, sampleRate)), 0)
    const frames = Math.max(0, Math.min(sampleRate, arrival) + own + Math.round((track.latencyOffsetMs ?? 0) * sampleRate / 1000))
    visiting.delete(id); memo.set(id, frames)
    return frames
  }
  return tracks.reduce((longest, track) => track.id === MASTER_TRACK || track.externalOutput ? Math.max(longest, output(track.id)) : longest, 0)
}

function reductionDb(slot: EffectSlot, levelDb: number): number | null {
  const params = slot.params
  if (params.type === "limiter") {
    if (!slot.enabled) return 0
    return Math.max(0, levelDb + params.inputGainDb - params.ceilingDb)
  }
  if (params.type === "compressor") {
    if (!slot.enabled) return 0
    const over = levelDb - params.thresholdDb
    return over > 0 ? over * (1 - 1 / params.ratio) : 0
  }
  return null
}

/**
 * One reading per compressor and limiter, in mixer order and then chain
 * order, as the engine lists them. `levelOf` is the level on a track, as
 * linear gain; the readings follow it, so the meters move with the beat.
 */
export function simulatedGainReductions(
  project: Project,
  levelOf: (trackIndex: number) => number
): GainReduction[] {
  return project.mixer.tracks.flatMap((track, index) => {
    const levelDb = gainToDb(levelOf(index))
    return track.effects.flatMap((slot): GainReduction[] => {
      if (
        project.plugins?.some(
          (plugin) =>
            plugin.target.type === "effect" && plugin.target.effect === slot.id
        )
      )
        return []
      const db = reductionDb(slot, levelDb)
      return db === null ? [] : [{ effect: slot.id, db }]
    })
  })
}
