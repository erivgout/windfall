import type { ChannelId, Note, PatternId, TimeSignature } from "@/bindings"
import { dispatch, useProjectStore } from "@/lib/store/project"

import { auditionOff, auditionOn } from "./audition"
import { Editor, type EditorContext, type EditorHost } from "./editor"
import { PianoRollSession } from "./session"
import { snapTicks } from "./snap"
import { usePianoRollStore } from "./store"

// An empty lane has no array in the project, and the editor tells lanes
// apart by reference, so every empty lane gets this one.
const EMPTY_NOTES: readonly Note[] = []

export function currentSignature(): TimeSignature {
  return useProjectStore.getState().project.settings.timeSignature
}

/** What the editor works on, read from the project as it is right now. */
export function readContext(
  patternId: PatternId,
  channelId: ChannelId
): EditorContext | null {
  const { project } = useProjectStore.getState()
  const pattern = project.patterns.find((item) => item.id === patternId)
  if (!pattern) return null
  if (!project.channels.some((channel) => channel.id === channelId)) return null
  const lane = pattern.lanes.find((item) => item.channel === channelId)
  return {
    pattern: {
      id: pattern.id,
      lengthSteps: pattern.lengthSteps,
      signature: project.settings.timeSignature,
    },
    channel: channelId,
    notes: lane?.notes ?? EMPTY_NOTES,
  }
}

/**
 * A session wired to the app: its editor dispatches to the project store,
 * reads its tool and snap from the piano roll's settings and auditions
 * through the backend.
 */
export function createSession(): PianoRollSession {
  const host: EditorHost = {
    dispatch: (command) => dispatch(command),
    context: () => {
      const editing = session.editing
      return editing ? readContext(editing.patternId, editing.channelId) : null
    },
    settings: () => {
      const state = usePianoRollStore.getState()
      return {
        tool: state.tool,
        snap: snapTicks(state.snap, currentSignature()),
        lastLength: state.lastLength,
        lastVelocity: state.lastVelocity,
      }
    },
    remember: (length, velocity) =>
      usePianoRollStore.getState().rememberNote(length, velocity),
    noteOn: auditionOn,
    noteOff: auditionOff,
  }
  const editor = new Editor(host)
  const session: PianoRollSession = new PianoRollSession(editor)
  // Actions and buttons ask the store whether anything is selected.
  editor.subscribe((event) => {
    if (event !== "selection" && event !== "scene") return
    const selectionCount = editor.selectionCount
    if (usePianoRollStore.getState().selectionCount !== selectionCount) {
      usePianoRollStore.setState({ selectionCount })
    }
  })
  return session
}
