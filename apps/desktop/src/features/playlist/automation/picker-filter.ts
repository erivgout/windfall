/** Automation ids matching the name or target words, in their original order. */
export function matchingAutomationIds<Id>(
  items: readonly { id: Id; name: string; target: string }[],
  query: string
): Id[] {
  const filter = query.trim().toLowerCase()
  return items
    .filter(
      (item) =>
        item.name.toLowerCase().includes(filter) ||
        item.target.toLowerCase().includes(filter)
    )
    .map((item) => item.id)
}
