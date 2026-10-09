type ProjectNameChange =
  { ok: true; name: string | null } | { ok: false; reason: "empty" | "long" }

export function projectNameChange(
  current: string,
  draft: string
): ProjectNameChange {
  const name = draft.trim()
  if (name === current) return { ok: true, name: null }
  if (name === "") return { ok: false, reason: "empty" }
  if (new TextEncoder().encode(name).length > 256) {
    return { ok: false, reason: "long" }
  }
  return { ok: true, name }
}
