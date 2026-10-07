import type { ClipContents } from "./edit-math"
import { usePianoRollStore } from "./store"

/*
 * The piano roll's own clipboard. It lives outside the panel, so notes
 * copied in one channel or pattern can be pasted into another. The store
 * mirrors its size so Paste can show whether there is anything to paste.
 */

let contents: ClipContents | null = null

export function writeClipboard(next: ClipContents | null) {
  contents = next
  usePianoRollStore.setState({ clipboardCount: next?.notes.length ?? 0 })
}

export function readClipboard(): ClipContents | null {
  return contents
}
