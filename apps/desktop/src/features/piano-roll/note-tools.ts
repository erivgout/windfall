import { create } from "zustand"

import type { Note, NoteEdge, NoteTransform } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { onProjectReplaced } from "@/lib/store/replaced"

import { currentSession } from "./session"

export type NoteTool = NoteTransform["type"]
export const NOTE_TOOLS: {
  value: NoteTool
  label: string
  description: string
}[] = [
  {
    value: "quantize",
    label: "Quantize",
    description:
      "Move starts or ends toward an original rhythmic grid. Lengths stay fixed when moving starts.",
  },
  {
    value: "legato",
    label: "Legato",
    description:
      "Reach the next selected onset. Notes in the last chord keep their lengths.",
  },
  {
    value: "staccato",
    label: "Staccato",
    description:
      "Shorten selected lengths by a percentage, keeping at least one tick.",
  },
  {
    value: "chop",
    label: "Chop",
    description:
      "Split at grid lines from pattern tick zero. Keep any partial first and last pieces.",
  },
  {
    value: "glue",
    label: "Glue",
    description:
      "Join touching or overlapping selected notes with the same pitch, velocity and pan.",
  },
  {
    value: "strum",
    label: "Strum",
    description:
      "Spread each selected chord with exactly matching starts, ordered by pitch. Lengths stay fixed.",
  },
  {
    value: "flipTime",
    label: "Flip time",
    description: "Mirror starts and ends across the selection's time span.",
  },
  {
    value: "flipPitch",
    label: "Flip pitch",
    description:
      "Mirror pitches between the selection's lowest and highest keys.",
  },
  {
    value: "keyRange",
    label: "Limit / transpose",
    description:
      "Transpose then place pitches in the inclusive MIDI range. Octave folding keeps pitch class where possible; otherwise it clamps.",
  },
  {
    value: "scaleVelocity",
    label: "Scale velocities",
    description:
      "Multiply selected velocities. Results stay between zero and full velocity.",
  },
]

export type ToolRequest = {
  tool: NoteTool
  edge: NoteEdge
  grid: number
  pattern: number
  channel: number
  notes: Note[]
  revision: number
}

export const useNoteTools = create<{ request: ToolRequest | null }>(() => ({
  request: null,
}))

/** Capture selection and document revision once, without changing the project. */
export function openNoteTools(tool: NoteTool, edge: NoteEdge = "start"): void {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (
    !context ||
    !session ||
    session.editor.busy ||
    notes.length === 0 ||
    notes.length !== session.editor.selectionCount
  )
    return
  useNoteTools.setState({
    request: {
      tool,
      edge,
      grid: session.editor.snapInterval || 240,
      pattern: context.pattern.id,
      channel: context.channel,
      notes: notes.map((note) => ({ ...note })),
      revision: useProjectStore.getState().revision,
    },
  })
}

export function closeNoteTools(): void {
  useNoteTools.setState({ request: null })
}

export function requestIsCurrent(request: ToolRequest): boolean {
  if (useNoteTools.getState().request !== request) return false
  const session = currentSession()
  const context = session?.editor.context
  return (
    !!session &&
    !session.editor.busy &&
    context?.pattern.id === request.pattern &&
    context.channel === request.channel &&
    useProjectStore.getState().revision === request.revision &&
    session.editor.selectionCount === request.notes.length &&
    request.notes.every((note) => session.editor.selection.has(note.id))
  )
}

export async function applyNoteTool(
  request: ToolRequest,
  transform: NoteTransform
): Promise<boolean> {
  if (!requestIsCurrent(request)) return false
  return currentSession()!.editor.transformNotes(request.notes, transform)
}

// A new document may reuse ids and revisions. Drop the captured request.
onProjectReplaced(closeNoteTools)
