import type { Channel, Clip, ClipId, MixerTrack, TrackId } from "@/bindings"
import { soloSet, targetsOf } from "@/lib/mixer-graph"
import { MASTER_TRACK } from "@/lib/units"

/*
 * The questions the strips ask about the mixer as a graph. The graph itself,
 * and the rule for what solo leaves audible, are in `lib/mixer-graph`.
 */

export { soloSet }

type Tracks = readonly MixerTrack[]

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
  /** Audio clips on the playlist that play into the track. */
  clips: readonly ClipId[]
  /** Tracks whose output is the track. */
  outputs: MixerTrack[]
  /** Tracks that send to the track. */
  sends: MixerTrack[]
}

/** Everything that plays into track `id`. */
export function feedersOf(
  tracks: Tracks,
  channels: readonly Channel[],
  clips: readonly Clip[],
  id: TrackId
): Feeders {
  const others = tracks.filter(
    (track) => track.id !== id && track.id !== MASTER_TRACK
  )
  return {
    channels: channels.filter((channel) => channel.mixerTrack === id),
    clips: audioClipsOfTrack(clips, id),
    outputs: others.filter((track) => track.output === id),
    sends: others.filter((track) =>
      track.sends.some((send) => send.target === id)
    ),
  }
}

const NO_CLIPS: readonly ClipId[] = []
const clipCache = new WeakMap<readonly Clip[], Map<TrackId, ClipId[]>>()

/**
 * The audio clips that play into a track, in timeline order, grouped once
 * per clip list. A track no clip plays into always gets the same empty
 * list, and a track gets the same list until a clip changes.
 */
export function audioClipsOfTrack(
  clips: readonly Clip[],
  id: TrackId
): readonly ClipId[] {
  let byTrack = clipCache.get(clips)
  if (!byTrack) {
    byTrack = new Map()
    const audio = clips
      .filter((clip) => clip.content.type === "audio")
      .sort((a, b) => a.start - b.start || a.id - b.id)
    for (const clip of audio) {
      if (clip.content.type !== "audio") continue
      const list = byTrack.get(clip.content.mixerTrack)
      if (list) list.push(clip.id)
      else byTrack.set(clip.content.mixerTrack, [clip.id])
    }
    clipCache.set(clips, byTrack)
  }
  return byTrack.get(id) ?? NO_CLIPS
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
