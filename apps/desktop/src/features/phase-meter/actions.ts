import { create } from "zustand"

import { registry, type Action } from "@/lib/actions"

export const usePhaseMeterPanel = create(() => ({ open: false }))

export function closePhaseMeter() {
  usePhaseMeterPanel.setState({ open: false })
}

const PHASE_METER_ACTIONS: Action[] = [
  {
    id: "view.phase-meter",
    title: "Show phase meter",
    section: "View",
    keywords: "stereo correlation side mid",
    run: () => usePhaseMeterPanel.setState({ open: true }),
  },
]

export function registerPhaseMeterActions(): () => void {
  const unregister = registry.register(PHASE_METER_ACTIONS)
  return () => {
    unregister()
    closePhaseMeter()
  }
}
