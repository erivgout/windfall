import type { PatternId } from "@/bindings"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { askText } from "@/lib/store/prompts"
import { selectedPatternId } from "@/lib/store/selectors"
import { setTransportPattern, useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"

function patterns() {
  return useProjectStore.getState().project.patterns
}

/** The pattern being edited, read right now. */
export function currentPatternId(): PatternId | null {
  return selectedPatternId(
    useProjectStore.getState().project,
    useTransportStore.getState().pattern
  )
}

export async function addPattern(): Promise<void> {
  const result = await dispatch({ type: "addPattern" })
  if (result) await setTransportPattern(result.created[0])
}

export async function duplicatePattern(id = currentPatternId()): Promise<void> {
  if (id === null) return
  const result = await dispatch({ type: "duplicatePattern", id })
  if (result) await setTransportPattern(result.created[0])
}

export async function renamePattern(id = currentPatternId()): Promise<void> {
  const pattern = patterns().find((item) => item.id === id)
  if (!pattern) return
  const name = await askText({
    title: "Rename pattern",
    label: "Name",
    initial: pattern.name,
    submitLabel: "Rename",
  })
  if (name === null || name === pattern.name) return
  await dispatch({ type: "updatePattern", id: pattern.id, patch: { name } })
}

export async function deletePattern(id = currentPatternId()): Promise<void> {
  if (id === null) return
  const index = patterns().findIndex((pattern) => pattern.id === id)
  const neighbor = patterns()[index + 1] ?? patterns()[index - 1]
  const wasSelected = id === currentPatternId()
  const result = await dispatch({ type: "removePattern", id })
  if (result && wasSelected && neighbor) await setTransportPattern(neighbor.id)
}

/** Selects the pattern after (1) or before (-1) the current one. */
export async function stepPattern(delta: 1 | -1): Promise<void> {
  const list = patterns()
  const index = list.findIndex((pattern) => pattern.id === currentPatternId())
  const next = list[index + delta]
  if (next) await setTransportPattern(next.id)
}

/** Adds an empty sampler channel, selects it and brings the rack forward. */
export async function addChannel(): Promise<void> {
  const result = await dispatch({ type: "addChannel" })
  if (!result) return
  const ui = useUiStore.getState()
  ui.selectChannel(result.created[0])
  ui.showCenterTab("channelRack")
}
