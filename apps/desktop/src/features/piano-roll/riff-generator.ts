import { create } from "zustand"

import type { Command, Note, NoteInit } from "@/bindings"
import {
  dispatch,
  onHistoryNavigation,
  useProjectStore,
} from "@/lib/store/project"
import { getProjectGeneration, onProjectReplaced } from "@/lib/store/replaced"
import { ticksPerBar } from "@/lib/time"
import { DEFAULT_KEY, MAX_PATTERN_TICKS, TICKS_PER_STEP } from "@/lib/units"

import type { EditorContext } from "./editor"
import { patternMeterAt } from "./pattern-timeline"
import { inScale, type PitchScale } from "./scales"
import { currentSession, type PianoRollSession } from "./session"
import { snapTicks } from "./snap"
import { usePianoRollStore } from "./store"

export const RIFF_SCALES = [
  { value: "major", label: "Major" },
  { value: "minor", label: "Natural minor" },
  { value: "major-pentatonic", label: "Pentatonic major" },
] as const
export const RIFF_DENSITIES = [
  { value: "low", label: "Low", chance: 0.25 },
  { value: "medium", label: "Medium", chance: 0.5 },
  { value: "high", label: "High", chance: 0.8 },
] as const
export const MAX_RIFF_SEED = 0xffffffff

export type RiffSettings = {
  root: number
  scale: (typeof RIFF_SCALES)[number]["value"]
  bars: number
  density: (typeof RIFF_DENSITIES)[number]["value"]
  seed: number
}
type Timing = { step: number; barTicks: number; velocity: number }

export function riffStart(
  notes: readonly Pick<Note, "start" | "length">[]
): number {
  return notes.reduce((end, note) => Math.max(end, note.start + note.length), 0)
}

/** One seeded melody in one scale; never rewrites its source notes. */
export function generateRiffNotes(
  source: readonly Pick<Note, "key" | "start" | "length">[],
  settings: RiffSettings,
  timing: Timing
): NoteInit[] {
  const density = RIFF_DENSITIES.find((item) => item.value === settings.density)
  if (
    !Number.isInteger(settings.root) ||
    settings.root < 0 ||
    settings.root > 11 ||
    !RIFF_SCALES.some((item) => item.value === settings.scale) ||
    !Number.isInteger(settings.bars) ||
    settings.bars < 1 ||
    settings.bars > 4 ||
    !density ||
    !Number.isInteger(settings.seed) ||
    settings.seed < 0 ||
    settings.seed > MAX_RIFF_SEED
  )
    throw new Error(
      "Choose a root, scale, 1–4 bars, density and a seed from 0 to 4294967295."
    )
  if (
    !Number.isInteger(timing.step) ||
    timing.step < 1 ||
    !Number.isInteger(timing.barTicks) ||
    timing.barTicks < timing.step ||
    !Number.isFinite(timing.velocity) ||
    timing.velocity < 0 ||
    timing.velocity > 1
  )
    throw new Error("The current snap or draw velocity is invalid.")

  const start = riffStart(source)
  const duration = settings.bars * timing.barTicks
  if (start + duration > MAX_PATTERN_TICKS)
    throw new Error(
      "There is not enough room after the channel's notes for this riff."
    )

  let low = 127
  let high = 0
  for (const note of source) {
    low = Math.min(low, note.key)
    high = Math.max(high, note.key)
  }
  const center = source.length ? Math.round((low + high) / 2) : DEFAULT_KEY
  const bottom = Math.max(0, Math.min(116, center - 6))
  const scale: PitchScale = { root: settings.root, id: settings.scale }
  const keys = Array.from({ length: 12 }, (_, index) => bottom + index).filter(
    (key) => inScale(key, scale)
  )
  let state = settings.seed >>> 0
  const random = () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    return state / 0x100000000
  }
  const notes: NoteInit[] = []
  for (
    let offset = 0;
    offset + timing.step <= duration;
    offset += timing.step
  ) {
    const hit = random()
    const key = keys[Math.floor(random() * keys.length)]
    // Always start with a note; the remaining cells may be rests.
    if (offset === 0 || hit < density.chance)
      notes.push({
        start: start + offset,
        length: timing.step,
        key,
        velocity: timing.velocity,
        pan: 0,
      })
  }
  return notes
}

export type RiffRequest = Timing & {
  context: EditorContext
  session: PianoRollSession
  revision: number
  generation: number
}
export const useRiffGenerator = create<{
  request: RiffRequest | null
  pending: boolean
}>(() => ({
  request: null,
  pending: false,
}))

export function openRiffGenerator(): void {
  const session = currentSession()
  const context = session?.editor.context
  if (
    !session ||
    !context ||
    session.editor.busy ||
    useRiffGenerator.getState().request
  )
    return
  const roll = usePianoRollStore.getState()
  const meter = patternMeterAt(
    riffStart(context.notes),
    context.pattern.signature,
    context.pattern.timeline
  )
  useRiffGenerator.setState({
    request: {
      context,
      session,
      revision: useProjectStore.getState().revision,
      generation: getProjectGeneration(),
      step: snapTicks(roll.snap, meter.signature) || TICKS_PER_STEP,
      barTicks: ticksPerBar(meter.signature),
      velocity: roll.lastVelocity,
    },
    pending: false,
  })
}

export function closeRiffGenerator(): void {
  useRiffGenerator.setState({ request: null, pending: false })
}

export function riffRequestIsCurrent(request: RiffRequest): boolean {
  const session = currentSession()
  return (
    useRiffGenerator.getState().request === request &&
    session === request.session &&
    !session.editor.busy &&
    session.editor.context?.pattern.id === request.context.pattern.id &&
    session.editor.context.channel === request.context.channel &&
    useProjectStore.getState().revision === request.revision &&
    getProjectGeneration() === request.generation
  )
}

export function riffCommand(
  request: RiffRequest,
  settings: RiffSettings
): Command {
  return {
    type: "addNotes",
    pattern: request.context.pattern.id,
    channel: request.context.channel,
    notes: generateRiffNotes(request.context.notes, settings, request),
  }
}

export async function applyRiff(
  request: RiffRequest,
  settings: RiffSettings
): Promise<boolean> {
  if (!riffRequestIsCurrent(request) || useRiffGenerator.getState().pending)
    return false
  const command = riffCommand(request, settings)
  useRiffGenerator.setState({ pending: true })
  try {
    const result = await dispatch(command)
    if (useRiffGenerator.getState().request === request && result)
      closeRiffGenerator()
    return result !== null
  } finally {
    if (useRiffGenerator.getState().request === request)
      useRiffGenerator.setState({ pending: false })
  }
}

onProjectReplaced(closeRiffGenerator)
onHistoryNavigation(closeRiffGenerator)
