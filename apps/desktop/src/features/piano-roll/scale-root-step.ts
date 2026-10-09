export function nextScaleRoot(
  root: number,
  direction: "lower" | "higher"
): number | null {
  if (!Number.isInteger(root) || root < 0 || root > 11) return null
  return (root + (direction === "lower" ? 11 : 1)) % 12
}
