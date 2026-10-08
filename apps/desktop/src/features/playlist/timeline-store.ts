import { create } from "zustand"
import type {
  MarkerKind,
  MeterChange,
  TickRange,
  TimelineMarker,
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
let request = 0
onProjectReplaced(() => {
  request += 1
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

/** Revision is captured before awaiting IPC. A delayed reply cannot arm another song. */
export async function applyPlaybackRegion(region: TickRange | null) {
  const sent = ++request
  const generation = getProjectGeneration()
  const revision = useProjectStore.getState().revision
  try {
    const state = await backend.timelineState()
    if (
      sent !== request ||
      generation !== getProjectGeneration() ||
      revision !== useProjectStore.getState().revision ||
      revision !== state.revision
    )
      return false
    await backend.timelineRegion(region, state.generation, state.revision)
    if (sent === request && generation === getProjectGeneration())
      useTimelineStore.setState({ active: region !== null, error: null })
    return sent === request && generation === getProjectGeneration()
  } catch (error) {
    if (sent === request && generation === getProjectGeneration())
      useTimelineStore.setState({
        error: error instanceof Error ? error.message : String(error),
      })
    return false
  }
}

export async function selectTimelineRegion(region: TickRange | null) {
  useTimelineStore.setState({ selection: region, error: null })
  if (useTimelineStore.getState().active) await applyPlaybackRegion(region)
}

export function editTimeline(edit: TimelineEdit) {
  useTimelineStore.setState({
    edit,
    selected: edit.item ? { type: edit.type, id: edit.item.id } : null,
  })
}
