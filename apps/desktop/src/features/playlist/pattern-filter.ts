/** Pattern ids whose names contain the trimmed query, in their original order. */
export function matchingPatternIds<Id>(
  patterns: readonly { id: Id; name: string }[],
  query: string
): Id[] {
  const filter = query.trim().toLowerCase()
  return patterns
    .filter((pattern) => pattern.name.toLowerCase().includes(filter))
    .map((pattern) => pattern.id)
}
