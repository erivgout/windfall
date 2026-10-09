import { create } from "zustand"

import { onProjectReplaced } from "@/lib/store/replaced"

// Separate from the shared dialog slot so Export can open without closing help.
export const useGettingStartedStore = create(() => ({ open: false }))

let stopReplacement: (() => void) | null = null

export function openGettingStarted() {
  stopReplacement ??= onProjectReplaced(closeGettingStarted)
  useGettingStartedStore.setState({ open: true })
}

export function closeGettingStarted() {
  stopReplacement?.()
  stopReplacement = null
  useGettingStartedStore.setState({ open: false })
}
