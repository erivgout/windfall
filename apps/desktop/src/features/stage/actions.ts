import { create } from "zustand"

import { registry, type Action } from "@/lib/actions"

type StageView = "clock" | "meter"

export const useStageView = create(() => ({ view: null as StageView | null }))

export function closeStageView() {
  useStageView.setState({ view: null })
}

const STAGE_ACTIONS: Action[] = [
  {
    id: "view.largeClock",
    title: "Large clock",
    section: "View",
    keywords: "stage transport time position bar beat step",
    run: () => useStageView.setState({ view: "clock" }),
  },
  {
    id: "view.largeMasterMeter",
    title: "Large master meter",
    section: "View",
    keywords: "stage stereo peak level decibels dB clip",
    run: () => useStageView.setState({ view: "meter" }),
  },
]

export function registerStageActions(): () => void {
  const unregister = registry.register(STAGE_ACTIONS)
  return () => {
    unregister()
    closeStageView()
  }
}
