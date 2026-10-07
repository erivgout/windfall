import type { Automation } from "@/bindings"
import {
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { CHANNEL_COLORS } from "@/features/channel-rack/color-swatches"

import type { PointRef } from "../store"
import { viewRangeItems } from "./view-menu"
import {
  automationById,
  deleteAutomation,
  deletePointAt,
  duplicateAutomation,
  renameAutomation,
  setAutomationColor,
  straightenPoint,
  togglePointHold,
  typePointValue,
} from "./ops"

/** What a right-click on a point of a curve offers. */
export function pointMenu(ref: PointRef): ContextItem[] {
  const automation = automationById(ref.automation)
  const point = automation?.points[ref.index]
  if (!automation || !point) return []
  const last = automation.points.length <= 1
  const leaves = ref.index < automation.points.length - 1
  return [
    { label: `Point of ${automation.name}` },
    { title: "Type value…", run: () => typePointValue(ref) },
    {
      title: "Hold",
      checked: point.hold,
      // The last point has no stretch after it to make a step of.
      disabled: !leaves,
      run: () => togglePointHold(ref),
    },
    {
      title: "Straighten",
      disabled: !leaves || (point.curve === 0 && !point.hold),
      run: () => straightenPoint(ref),
    },
    contextSeparator,
    {
      title: "Delete",
      destructive: true,
      disabled: last,
      reason: "The only point",
      run: () => deletePointAt(ref),
    },
  ]
}

/** What a right-click on an automation in the list offers. */
export function automationMenu(automation: Automation): ContextItem[] {
  const id = automation.id
  return [
    { title: "Rename…", run: () => renameAutomation(id) },
    {
      submenu: "Color",
      items: CHANNEL_COLORS.map(({ color, name }) => ({
        title: name,
        checked: automation.color === color,
        run: () => setAutomationColor(id, color),
      })),
    },
    { title: "Duplicate", run: () => duplicateAutomation(id) },
    ...viewRangeItems([id]),
    contextSeparator,
    {
      title: "Delete…",
      destructive: true,
      run: () => deleteAutomation(id),
    },
  ]
}
