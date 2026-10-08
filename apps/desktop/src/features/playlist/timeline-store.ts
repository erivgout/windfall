import { create } from "zustand"
import type {
  MarkerKind,
  MeterChange,
  TickRange,
  TimelineMarker,
  TimelinePlaybackState,
} from "@/bindings"
import { backend } from "@/lib/ipc"
import { useProjectStore } from "@/lib/store/project"
import { getProjectGeneration, onProjectReplaced } from "@/lib/store/replaced"

export type RulerTool = "seek" | "select" | "zoom"
export type TimelineEdit =
  | { type: "meter"; item?: MeterChange }
  | { type: "marker"; kind: MarkerKind; item?: TimelineMarker }
export const useTimelineStore = create(() => ({
  tool: "seek" as RulerTool,
  selection: null as TickRange | null,
  draft: null as TickRange | null,
  active: false,
  exportSelection: false,
  error: null as string | null,
  selected: null as { type: "meter" | "marker"; id: number } | null,
  edit: null as TimelineEdit | null,
}))
let currentRequest: TimelineRequest | null = null
let wireRequest = 0
// A refused/edited publication may already have armed native transport.
// This schedules reconciliation; canonical guards remain the authority.
let mayBeArmed = false
const pending = new Set<TimelineRequest>()
type TimelineRequest = {
  generation: number
  revision: number
  current(): boolean
}
export function beginTimelineRequest(): TimelineRequest {
  const generation = getProjectGeneration()
  const revision = useProjectStore.getState().revision
  const operation = {
    generation,
    current: () =>
      currentRequest === operation &&
      generation === getProjectGeneration() &&
      revision === useProjectStore.getState().revision,
    revision,
  }
  currentRequest = operation
  return operation
}
onProjectReplaced(() => {
  currentRequest = null
  mayBeArmed = false
  useTimelineStore.setState({
    tool: "seek",
    selection: null,
    draft: null,
    active: false,
    exportSelection: false,
    error: null,
    selected: null,
    edit: null,
  })
})
useProjectStore.subscribe(({ project }) => {
  const selected = useTimelineStore.getState().selected
  const items =
    selected?.type === "meter"
      ? project.playlist.timeline?.meters
      : project.playlist.timeline?.markers
  if (selected && !items?.some((item) => item.id === selected.id))
    useTimelineStore.setState({ selected: null, edit: null })
})

export function timelineRequestError(
  operation: TimelineRequest,
  error: unknown
) {
  if (operation.current())
    useTimelineStore.setState({
      error: error instanceof Error ? error.message : String(error),
    })
}

/** Native request ordering reconciles publications even when their replies arrive late. */
export async function publishPlaybackRegion(
  region: TickRange | null,
  operation: TimelineRequest
): Promise<TimelinePlaybackState | null> {
  pending.add(operation)
  try {
    const state = await backend.timelineState()
    if (!operation.current() || operation.revision !== state.revision)
      return null
    mayBeArmed ||= state.region !== null
    if (!Number.isSafeInteger(state.request) || state.request < 0)
      throw new Error(
        "The backend returned an invalid timeline request number."
      )
    wireRequest = Math.max(wireRequest, state.request)
    if (wireRequest >= Number.MAX_SAFE_INTEGER)
      throw new Error(
        "The timeline request limit was reached. Restart the app before selecting another playback region."
      )
    mayBeArmed ||= region !== null
    const published = await backend.timelineRegion(
      region,
      state.generation,
      state.revision,
      ++wireRequest
    )
    if (!operation.current()) return null
    mayBeArmed = region !== null
    useTimelineStore.setState({ active: region !== null, error: null })
    return published
  } catch (error) {
    timelineRequestError(operation, error)
    return null
  } finally {
    pending.delete(operation)
  }
}

export async function applyPlaybackRegion(region: TickRange | null) {
  return (await publishPlaybackRegion(region, beginTimelineRequest())) !== null
}

export async function selectTimelineRegion(region: TickRange | null) {
  const needsPublication =
    mayBeArmed ||
    useTimelineStore.getState().active ||
    [...pending].some(
      (operation) => operation.generation === getProjectGeneration()
    )
  currentRequest = null
  useTimelineStore.setState({ selection: region, active: false, error: null })
  if (needsPublication) await applyPlaybackRegion(region)
}

export function editTimeline(edit: TimelineEdit) {
  useTimelineStore.setState({
    edit,
    selected: edit.item ? { type: edit.type, id: edit.item.id } : null,
  })
}
