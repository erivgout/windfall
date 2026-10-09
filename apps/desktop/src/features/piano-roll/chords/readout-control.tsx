import { useSyncExternalStore } from "react"

import { useSession } from "../context"
import { chordReadout } from "./readout"

export function ChordReadout() {
  const session = useSession()
  const { editor } = session
  const label = useSyncExternalStore(
    (listener) => {
      const stopEditor = editor.subscribe(listener)
      const stopPlayhead = session.onPlayhead(listener)
      return () => {
        stopEditor()
        stopPlayhead()
      }
    },
    () => chordReadout(editor.notes, editor.selection, session.playhead)
  )

  return (
    <span
      aria-label="Chord"
      className="shrink-0 rounded-sm bg-display px-2 py-1 font-readout text-[0.6875rem] text-display-foreground"
    >
      {label}
    </span>
  )
}
