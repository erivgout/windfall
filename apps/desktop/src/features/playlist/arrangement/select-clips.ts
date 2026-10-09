import type { ArrangementBook, ClipId } from "@/bindings"

/** Returns distinct live clips in active arrangement order, or null if absent. */
export function activeArrangementClipIds(
  book: ArrangementBook,
  liveClipIds: Iterable<ClipId>
): ClipId[] | null {
  if (book.active === null) return null
  const arrangement = book.arrangements.find((item) => item.id === book.active)
  if (!arrangement) return null

  const live = new Set(liveClipIds)
  const ids = new Set<ClipId>()
  for (const id of arrangement.clips) {
    if (live.has(id)) ids.add(id)
  }
  return [...ids]
}
