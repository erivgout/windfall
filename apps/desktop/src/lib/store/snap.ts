import { create } from "zustand"

import { onProjectReplaced } from "./replaced"

export type SharedSnap = "none" | "step" | "beat" | "bar"

export function isSharedSnap(value: unknown): value is SharedSnap {
  return value === "none" || value === "step" || value === "beat" || value === "bar"
}

type SnapState = {
  snap: SharedSnap
  setSnap(snap: SharedSnap): void
}

/** Session choice shared by the playlist and piano roll, outside either panel. */
export const useSnapStore = create<SnapState>()((set) => ({
  snap: "bar",
  // Notify even when the value is unchanged: choosing it again clears the
  // piano roll's local division through its subscription.
  setSnap: (snap) => set({ snap }),
}))

onProjectReplaced(() => useSnapStore.getState().setSnap("bar"))
