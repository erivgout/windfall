import { create } from "zustand"

import type { AutomationId } from "@/bindings"
import {
  extendView,
  inView,
  sameView,
  type ViewRange,
} from "@/lib/automation/view-range"
import { onProjectReplaced } from "@/lib/store/replaced"

type CurveViews = {
  /**
   * The view ranges chosen for automations, by id. An automation that is
   * not in here draws in its default view.
   */
  views: Readonly<Record<AutomationId, ViewRange>>
  setView(automation: AutomationId, view: ViewRange | null): void
}

/**
 * How much of its range each automation's clips show. This is a way of
 * looking at a curve, not a part of the song: it is kept with the rest of
 * the playlist's view state, not in the project, and undo leaves it alone.
 *
 * It is kept for as long as the project is open. Ids start over in every
 * project, so a view kept past that would land on some other curve.
 */
export const useCurveViews = create<CurveViews>()((set, get) => ({
  views: {},
  setView: (automation, view) => {
    const current = get().views
    const before = current[automation]
    if (view === null) {
      if (before === undefined) return
      const rest = { ...current }
      delete rest[automation]
      set({ views: rest })
      return
    }
    if (before && sameView(before, view)) return
    set({ views: { ...current, [automation]: view } })
  },
}))

onProjectReplaced(() => useCurveViews.setState({ views: {} }))

/** The view chosen for an automation, if one was. */
export function chosenView(automation: AutomationId): ViewRange | undefined {
  return useCurveViews.getState().views[automation]
}

/**
 * Grows the view chosen for an automation to show these values too: a
 * point taken past the edge of the view takes the edge with it. A curve in
 * its default view needs nothing, that view follows its points by itself.
 */
export function showInView(
  automation: AutomationId,
  values: readonly number[]
): void {
  const chosen = chosenView(automation)
  if (!chosen || values.every((value) => inView(chosen, value))) return
  useCurveViews.getState().setView(automation, extendView(chosen, ...values))
}
