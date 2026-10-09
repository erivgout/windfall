import type { BrowserRoot } from "@/bindings"

import { isUnder, rowId, type Listings } from "./tree-model"

/** Root rows and folder rows already known from ready listings. */
export function loadedFolderIds(
  roots: readonly BrowserRoot[],
  listings: Listings
): Set<string> {
  const ids = new Set(roots.map((root) => rowId(root.path, root.path)))
  for (const listing of Object.values(listings)) {
    if (listing?.status !== "ready") continue
    for (const entry of listing.entries) {
      if (entry.kind !== "folder") continue
      for (const root of roots) {
        if (entry.path === root.path || isUnder(entry.path, root.path)) {
          ids.add(rowId(root.path, entry.path))
        }
      }
    }
  }
  return ids
}
