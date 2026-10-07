import { useHintStore } from "@/lib/store/hint"

/*
 * The status-bar hint for the thing under the pointer. `useHint` covers
 * controls with a fixed hint; the grid's hint changes as the pointer moves
 * over notes, edges and empty space, so it is set from here.
 */

let shown: string | null = null

export function showHint(text: string | null) {
  if (text === shown) return
  const store = useHintStore
  if (text !== null) {
    store.setState({ text })
  } else if (store.getState().text === shown) {
    // Only clear a hint that is still ours.
    store.setState({ text: null })
  }
  shown = text
}
