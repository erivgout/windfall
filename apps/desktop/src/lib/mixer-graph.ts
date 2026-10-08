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
  if (track.id === MASTER_TRACK || track.current) return []
  const targets = track.sends.map((send) => send.target)
  if (track.output !== null && !track.externalOutput?.exclusive) targets.push(track.output)
  return targets.filter((target) => target !== track.id && known.has(target))
}

/** All ordering dependencies, including detector-only routes. */
export function dependencyTargets(track: MixerTrack, known: ReadonlySet<TrackId>): TrackId[] {
  return [...targetsOf(track, known), ...(track.id === MASTER_TRACK || track.current ? [] : (track.sidechains ?? []).map((send) => send.target).filter((id) => id !== track.id && known.has(id)))]
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
  const soloed = tracks.filter((track) => track.solo && !track.current).map((track) => track.id)
  if (soloed.length === 0) return ids

  const downstream = new Map<TrackId, TrackId[]>()
  const upstream = new Map<TrackId, TrackId[]>()
  for (const track of tracks) {
    const targets = targetsOf(track, ids)
    downstream.set(track.id, targets)
    for (const target of dependencyTargets(track, ids)) {
      upstream.set(target, [...(upstream.get(target) ?? []), track.id])
    }
  }
  return new Set([...spread(soloed, downstream), ...spread(soloed, upstream), ...tracks.filter((track) => track.current).map((track) => track.id)])
}
