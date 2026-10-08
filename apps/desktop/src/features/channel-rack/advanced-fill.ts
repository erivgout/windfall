import { create } from "zustand"

import type { ChannelId, Command, Note, NoteInit, PatternId } from "@/bindings"
import {
  dispatch,
  onHistoryNavigation,
  useProjectStore,
} from "@/lib/store/project"
import { getProjectGeneration, onProjectReplaced } from "@/lib/store/replaced"
import { selectedPatternId } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

export type FillRule = "regular" | "euclidean" | "random"
export type FillOptions = {
  rule: FillRule
  startStep: number
  endStep: number
  cycle: number
  every: number
  hits: number
  rotation: number
  seed: number
  key: number
  velocity: number
  gate: number
  replace: boolean
}

const integer = (value: number, min: number, max: number) =>
  Number.isInteger(value) && value >= min && value <= max

export function validFillOptions(
  options: FillOptions,
  lengthSteps: number
): boolean {
  return (
    integer(lengthSteps, 1, MAX_PATTERN_STEPS) &&
    ["regular", "euclidean", "random"].includes(options.rule) &&
    integer(options.startStep, 0, lengthSteps - 1) &&
    integer(options.endStep, options.startStep + 1, lengthSteps) &&
    integer(options.cycle, 1, lengthSteps) &&
    integer(options.rotation, 0, options.cycle - 1) &&
    (options.rule === "regular"
      ? integer(options.every, 1, options.cycle)
      : integer(options.hits, 0, options.cycle)) &&
    (options.rule !== "random" || integer(options.seed, 0, 0xffffffff)) &&
    integer(options.key, 0, 127) &&
    Number.isFinite(options.velocity) &&
    options.velocity >= 0 &&
    options.velocity <= 1 &&
    Number.isFinite(options.gate) &&
    options.gate > 0 &&
    options.gate <= 1 &&
    typeof options.replace === "boolean"
  )
}

/** One bounded cycle. Seeded shuffle chooses exactly `hits` distinct steps. */
function rhythm(options: FillOptions): Set<number> {
  const { rule, cycle, hits, every, rotation } = options
  let steps: number[]
  if (rule === "regular") {
    steps = Array.from(
      { length: Math.ceil(cycle / every) },
      (_, i) => i * every
    )
  } else if (rule === "euclidean") {
    // Modular distribution starts at zero and has cyclic gaps differing
    // by at most one step, including non-divisible pulse/step counts.
    steps = Array.from({ length: cycle }, (_, i) => i).filter(
      (i) => (i * hits) % cycle < hits
    )
  } else {
    steps = Array.from({ length: cycle }, (_, i) => i)
    let state = options.seed >>> 0
    const next = () => {
      state = (state + 0x6d2b79f5) >>> 0
      let value = Math.imul(state ^ (state >>> 15), 1 | state)
      value ^= value + Math.imul(value ^ (value >>> 7), 61 | value)
      return ((value ^ (value >>> 14)) >>> 0) / 0x100000000
    }
    for (let i = cycle - 1; i > 0; i -= 1) {
      const j = Math.floor(next() * (i + 1))
      ;[steps[i], steps[j]] = [steps[j], steps[i]]
    }
    steps = steps.slice(0, hits)
  }
  return new Set(steps.map((step) => (step + rotation) % cycle))
}

/** Authored preview notes are also the exact payload reviewed by the user. */
export function fillPreview(
  options: FillOptions,
  lengthSteps: number
): NoteInit[] | null {
  if (!validFillOptions(options, lengthSteps)) return null
  const cycle = rhythm(options)
  const notes: NoteInit[] = []
  for (let step = options.startStep; step < options.endStep; step += 1) {
    if (!cycle.has((step - options.startStep) % options.cycle)) continue
    notes.push({
      start: step * TICKS_PER_STEP,
      length: Math.max(1, Math.round(TICKS_PER_STEP * options.gate)),
      key: options.key,
      velocity: options.velocity,
      pan: 0,
    })
  }
  return notes
}

export type FillRequest = {
  id: number
  generation: number
  revision: number
  pattern: PatternId
  channel: ChannelId
  patternName: string
  channelName: string
  lengthSteps: number
  notes: Note[]
}

let nextRequest = 0
export const useAdvancedFill = create<{ request: FillRequest | null }>(() => ({
  request: null,
}))

export function openAdvancedFill(): void {
  const { project, revision } = useProjectStore.getState()
  const channel = project.channels.find(
    (item) => item.id === useUiStore.getState().selectedChannel
  )
  const patternId = selectedPatternId(
    project,
    useTransportStore.getState().pattern
  )
  const pattern = project.patterns.find((item) => item.id === patternId)
  if (!channel || !pattern) return
  useUiStore.getState().showCenterTab("channelRack")
  useAdvancedFill.setState({
    request: {
      id: ++nextRequest,
      generation: getProjectGeneration(),
      revision,
      pattern: pattern.id,
      channel: channel.id,
      patternName: pattern.name,
      channelName: channel.name,
      lengthSteps: pattern.lengthSteps,
      notes: (
        pattern.lanes.find((lane) => lane.channel === channel.id)?.notes ?? []
      ).map((note) => ({ ...note })),
    },
  })
}

export function closeAdvancedFill(): void {
  useAdvancedFill.setState({ request: null })
}

export function fillRequestIsCurrent(request: FillRequest): boolean {
  const state = useProjectStore.getState()
  return (
    useAdvancedFill.getState().request === request &&
    request.generation === getProjectGeneration() &&
    request.revision === state.revision &&
    request.channel === useUiStore.getState().selectedChannel &&
    request.pattern ===
      selectedPatternId(state.project, useTransportStore.getState().pattern) &&
    state.project.channels.some((channel) => channel.id === request.channel) &&
    state.project.patterns.some(
      (pattern) =>
        pattern.id === request.pattern &&
        pattern.lengthSteps === request.lengthSteps
    )
  )
}

export function fillRangeCommand(
  request: FillRequest,
  options: FillOptions
): Command | null {
  const notes = fillPreview(options, request.lengthSteps)
  if (!notes) return null
  return {
    type: "fillStepRange",
    pattern: request.pattern,
    channel: request.channel,
    lengthSteps: request.lengthSteps,
    expected: request.notes,
    startStep: options.startStep,
    endStep: options.endStep,
    replace: options.replace,
    notes,
  }
}

// One submission for the captured request, even before React renders pending.
const pending = new Set<FillRequest>()
export async function applyAdvancedFill(
  request: FillRequest,
  options: FillOptions
): Promise<boolean> {
  if (!fillRequestIsCurrent(request) || pending.has(request)) return false
  const command = fillRangeCommand(request, options)
  if (!command) return false
  pending.add(request)
  try {
    const result = await dispatch(command)
    if (!result) return false
    if (useAdvancedFill.getState().request === request) closeAdvancedFill()
    return true
  } finally {
    pending.delete(request)
  }
}

onProjectReplaced(closeAdvancedFill)
onHistoryNavigation(closeAdvancedFill)
