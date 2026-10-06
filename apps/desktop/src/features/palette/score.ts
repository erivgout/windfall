/**
 * Letters scattered across a whole title match almost anything, so a loose
 * match must have its letters at least this close together.
 */
const MIN_TIGHTNESS = 0.3

/**
 * How well a search matches an action, from 0 (hide it) to 1. The title
 * counts most: a search that starts it or starts its words beats one that
 * is merely inside it, which beats a match in the keywords, which beats
 * letters that only appear in order ("sv as" for "Save as").
 */
export function scoreAction(
  title: string,
  keywords: string,
  search: string
): number {
  const query = search.trim().toLowerCase()
  if (query === "") return 1

  const name = title.toLowerCase()
  if (name.startsWith(query)) return 1

  const nameWords = name.split(/\s+/)
  const queryWords = query.split(/\s+/)
  const startsWords = queryWords.every((word) =>
    nameWords.some((candidate) => candidate.startsWith(word))
  )
  if (startsWords) return 0.9
  if (name.includes(query)) return 0.8

  const extraWords = keywords.toLowerCase().split(/\s+/)
  const inKeywords = queryWords.every((word) =>
    [...nameWords, ...extraWords].some((candidate) =>
      candidate.startsWith(word)
    )
  )
  if (inKeywords) return 0.6

  const loose = inOrder(name, query.replace(/\s+/g, ""))
  return loose < MIN_TIGHTNESS ? 0 : 0.2 + 0.3 * loose
}

/**
 * Whether the letters of `query` appear in `text` in order. Returns 0 when
 * they do not, and otherwise how tightly they sit together, up to 1.
 */
function inOrder(text: string, query: string): number {
  let from = text.indexOf(query[0])
  if (from < 0) return 0
  const first = from
  for (const letter of query.slice(1)) {
    from = text.indexOf(letter, from + 1)
    if (from < 0) return 0
  }
  return query.length / (from - first + 1)
}
