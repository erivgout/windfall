import type { MixerTrack, TrackId } from "@/bindings"
import { MASTER_TRACK } from "@/lib/units"

/*
 * The mixer as a graph: a track feeds its output and the targets of its
 * sends. This mirrors `compile_mixer` and `solo_set` in the engine, so the
 * mixer panel and the mock backend's meters both go silent where the engine
 * does.
 */

/** The tracks `track` feeds. The master feeds nothing, whatever it stores. */
export function targetsOf(
  track: MixerTrack,
  known: ReadonlySet<TrackId>
): TrackId[] {
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
export function soloSet(tracks: readonly MixerTrack[]): Set<TrackId> {
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
