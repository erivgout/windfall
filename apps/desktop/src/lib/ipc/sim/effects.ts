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

/**
 * A stand-in for the engine's latency figure: the slowest way from a
 * channel to the master, counting built-in effect latency and the synth's
 * own delay. Sends are left out. This is UI simulation, not browser DSP.
 */
export function simulatedLatencyFrames(
  project: Project,
  sampleRate: number
): number {
  const tracks = project.mixer.tracks
  const toOutput = (id: TrackId, depth: number): number => {
    const track = tracks.find((item) => item.id === id)
    if (!track || depth > tracks.length) return 0
    const own = track.effects.reduce(
      (sum, slot) =>
        sum +
        (project.plugins?.some(
          (plugin) =>
            plugin.target.type === "effect" && plugin.target.effect === slot.id
        )
          ? 0
          : slotLatency(slot, sampleRate)),
      0
    )
    if (track.id === MASTER_TRACK || track.output === null) return own
    return own + toOutput(track.output, depth + 1)
  }

  let longest = toOutput(MASTER_TRACK, 0)
  for (const track of tracks) {
    longest = Math.max(longest, toOutput(track.id, 0))
  }
  for (const channel of project.channels) {
    if (channel.source.type !== "instrument") continue
    if (
      project.plugins?.some(
        (plugin) =>
          plugin.target.type === "instrument" &&
          plugin.target.channel === channel.id
      )
    )
      continue
    longest = Math.max(
      longest,
      INSTRUMENT_LATENCY_FRAMES + toOutput(channel.mixerTrack, 0)
    )
  }
  return longest
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
