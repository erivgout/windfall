const KEY = "windfall.browser"
const SCROLL_SAVE_MS = 200

export type Persisted = {
  /** Row ids of open folders. Null until the browser has run once. */
  expanded: string[] | null
  autoPreview: boolean
  scrollTop: number
}

const DEFAULTS: Persisted = { expanded: null, autoPreview: true, scrollTop: 0 }

function storage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage
  } catch {
    return null
  }
}

/** What the browser remembered from last time. Anything unreadable is ignored. */
export function readPersisted(): Persisted {
  try {
    const text = storage()?.getItem(KEY)
    if (!text) return DEFAULTS
    const value: unknown = JSON.parse(text)
    if (typeof value !== "object" || value === null) return DEFAULTS
    const saved = value as Record<string, unknown>
    return {
      expanded: Array.isArray(saved.expanded)
        ? saved.expanded.filter((id): id is string => typeof id === "string")
        : null,
      autoPreview:
        typeof saved.autoPreview === "boolean"
          ? saved.autoPreview
          : DEFAULTS.autoPreview,
      scrollTop:
        typeof saved.scrollTop === "number" && saved.scrollTop >= 0
          ? saved.scrollTop
          : 0,
    }
  } catch {
    return DEFAULTS
  }
}

export function writePersisted(patch: Partial<Persisted>) {
  try {
    storage()?.setItem(KEY, JSON.stringify({ ...readPersisted(), ...patch }))
  } catch {
    // A full or blocked storage only costs the remembered layout.
  }
}

let scrollTimer: ReturnType<typeof setTimeout> | null = null
let pendingScrollTop: number | null = null

/** Saves the scroll position soon, without writing on every scroll event. */
export function saveScrollTop(scrollTop: number) {
  pendingScrollTop = scrollTop
  scrollTimer ??= setTimeout(flushScrollTop, SCROLL_SAVE_MS)
}

export function flushScrollTop() {
  if (scrollTimer !== null) clearTimeout(scrollTimer)
  scrollTimer = null
  if (pendingScrollTop === null) return
  writePersisted({ scrollTop: pendingScrollTop })
  pendingScrollTop = null
}

/** Forgets a save that has not been written yet. For tests. */
export function dropPendingScrollTop() {
  if (scrollTimer !== null) clearTimeout(scrollTimer)
  scrollTimer = null
  pendingScrollTop = null
}
