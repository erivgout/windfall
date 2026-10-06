import type { Channel, MixerTrack, TrackId } from "@/bindings"
import { MASTER_TRACK } from "@/lib/units"

/*
 * The mixer as a graph: a track feeds its output and the targets of its
 * sends. These helpers answer the questions the strips ask about it. They
 * mirror `compile_mixer` in the engine, so what the mixer shows as silent is
 * what the engine plays as silent.
 */

type Tracks = readonly MixerTrack[]

/** The tracks `track` feeds. The master feeds nothing, whatever it stores. */
function targetsOf(track: MixerTrack, known: ReadonlySet<TrackId>): TrackId[] {
  if (track.id === MASTER_TRACK) return []
  const targets = track.sends.map((send) => send.target)
  if (track.output !== null) targets.push(track.output)
  return targets.filter((target) => target !== track.id && known.has(target))
}

function spread(
  from: Iterable<TrackId>,
  neighbours: ReadonlyMap<TrackId, readonly TrackId[]>
): Set<TrackId> {
  const reached = new Set(from)
  const pending = [...reached]
  for (let id = pending.pop(); id !== undefined; id = pending.pop()) {
    for (const next of neighbours.get(id) ?? []) {
      if (!reached.has(next)) {
        reached.add(next)
        pending.push(next)
      }
    }
  }
  return reached
}

/**
 * The tracks solo leaves audible: all of them when nothing is soloed.
 * Otherwise a track is heard if it is soloed, feeds a soloed track through
 * outputs and sends, or carries a soloed track's signal on toward the
 * master. Mute is not looked at here; it silences a track on top of this.
 */
export function soloSet(tracks: Tracks): Set<TrackId> {
  const ids = new Set(tracks.map((track) => track.id))
  const soloed = tracks.filter((track) => track.solo).map((track) => track.id)
  if (soloed.length === 0) return ids

  const downstream = new Map<TrackId, TrackId[]>()
  const upstream = new Map<TrackId, TrackId[]>()
  for (const track of tracks) {
    const targets = targetsOf(track, ids)
    downstream.set(track.id, targets)
    for (const target of targets) {
      upstream.set(target, [...(upstream.get(target) ?? []), track.id])
    }
  }
  return new Set([...spread(soloed, downstream), ...spread(soloed, upstream)])
}

const heardCache = new WeakMap<Tracks, Set<TrackId>>()

/**
 * `soloSet`, worked out once per track list. The store hands out a new list
 * only when a track changed, so every strip can ask on every store update.
 */
export function heardTracks(tracks: Tracks): Set<TrackId> {
  let heard = heardCache.get(tracks)
  if (!heard) {
    heard = soloSet(tracks)
    heardCache.set(tracks, heard)
  }
  return heard
}

export type Audibility =
  /** Plays. */
  | "heard"
  /** Its own mute is on. Mute wins over solo. */
  | "muted"
  /** Not muted, but another track's solo leaves it out. */
  | "silenced"

export function audibility(tracks: Tracks, id: TrackId): Audibility {
  const track = tracks.find((item) => item.id === id)
  if (track?.muted) return "muted"
  return heardTracks(tracks).has(id) ? "heard" : "silenced"
}

/** True when audio leaving `from` can arrive at `to`. A track reaches itself. */
export function reaches(tracks: Tracks, from: TrackId, to: TrackId): boolean {
  const ids = new Set(tracks.map((track) => track.id))
  const seen = new Set<TrackId>()
  const pending = [from]
  for (let id = pending.pop(); id !== undefined; id = pending.pop()) {
    if (id === to) return true
    if (seen.has(id)) continue
    seen.add(id)
    const track = tracks.find((item) => item.id === id)
    if (track) pending.push(...targetsOf(track, ids))
  }
  return false
}

export type RoutingChoices = {
  /** Tracks that can be picked, in mixer order. */
  tracks: MixerTrack[]
  /** How many tracks were left out because picking them would loop back. */
  looping: number
}

/**
 * Where track `id` may send its output: every other track that does not
 * already lead back to it. The master leads nowhere, so it is always there.
 */
export function outputChoices(tracks: Tracks, id: TrackId): RoutingChoices {
  if (id === MASTER_TRACK) return { tracks: [], looping: 0 }
  const others = tracks.filter((track) => track.id !== id)
  const open = others.filter((track) => !reaches(tracks, track.id, id))
  return { tracks: open, looping: others.length - open.length }
}

/** Where track `id` may add a send: like its output, minus the sends it has. */
export function sendChoices(tracks: Tracks, id: TrackId): RoutingChoices {
  const track = tracks.find((item) => item.id === id)
  if (!track) return { tracks: [], looping: 0 }
  const taken = new Set(track.sends.map((send) => send.target))
  const { tracks: open, looping } = outputChoices(tracks, id)
  return { tracks: open.filter((item) => !taken.has(item.id)), looping }
}

export type Feeders = {
  /** Channels that play into the track. */
  channels: Channel[]
  /** Tracks whose output is the track. */
  outputs: MixerTrack[]
  /** Tracks that send to the track. */
  sends: MixerTrack[]
}

/** Everything that plays into track `id`. */
export function feedersOf(
  tracks: Tracks,
  channels: readonly Channel[],
  id: TrackId
): Feeders {
  const others = tracks.filter(
    (track) => track.id !== id && track.id !== MASTER_TRACK
  )
  return {
    channels: channels.filter((channel) => channel.mixerTrack === id),
    outputs: others.filter((track) => track.output === id),
    sends: others.filter((track) =>
      track.sends.some((send) => send.target === id)
    ),
  }
}

/** How many tracks play into track `id` through an output or a send. */
export function trackInputCount(tracks: Tracks, id: TrackId): number {
  let count = 0
  for (const track of tracks) {
    if (track.id === id || track.id === MASTER_TRACK) continue
    if (track.output === id || track.sends.some((send) => send.target === id)) {
      count += 1
    }
  }
  return count
}

const NO_CHANNELS: Channel[] = []
const channelCache = new WeakMap<readonly Channel[], Map<TrackId, Channel[]>>()

/**
 * The channels that play into a track, grouped once per channel list. A
 * track nothing plays into always gets the same empty list.
 */
export function channelsOfTrack(
  channels: readonly Channel[],
  id: TrackId
): Channel[] {
  let byTrack = channelCache.get(channels)
  if (!byTrack) {
    byTrack = new Map()
    for (const channel of channels) {
      const list = byTrack.get(channel.mixerTrack)
      if (list) list.push(channel)
      else byTrack.set(channel.mixerTrack, [channel])
    }
    channelCache.set(channels, byTrack)
  }
  return byTrack.get(id) ?? NO_CHANNELS
}

/** The most sends any one track has. */
export function maxSendCount(tracks: Tracks): number {
  return tracks.reduce((most, track) => Math.max(most, track.sends.length), 0)
}
