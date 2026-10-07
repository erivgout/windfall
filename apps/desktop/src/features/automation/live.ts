import { useMemo } from "react"

import type { AutomationTarget, ParamInfo } from "@/bindings"
import type { LiveValueFeed } from "@/components/audio"
import type { ContextItem } from "@/components/context-actions"
import { targetKey } from "@/lib/automation/lanes"
import { useProjectStore } from "@/lib/store/project"
import { subscribeRealtime } from "@/lib/store/realtime"
import { colorToCss } from "@/lib/units"

import { automationItems } from "./menu"
import { automatedNow } from "./now"

export { automatedNow }

/*
 * What automation is doing to a control right now, and whether it has any.
 * The engine reports, 60 times a second, the value each automation that
 * has its target in hand is giving it. A control bound to that target
 * shows the value while the song plays; the stored value stays what a drag
 * changes.
 */

/**
 * A feed of what automation is doing to a target, in the shape the kit's
 * knobs and faders take as their `live` prop. The listener is called when
 * the value changes, and with null when the target is let go of.
 */
export function automationFeed(target: AutomationTarget): LiveValueFeed {
  return (listener) => {
    let last: number | null = null
    return subscribeRealtime(() => {
      const now = automatedNow(target)
      const value = now?.value ?? null
      if (value === last) return
      last = value
      listener(value, now?.color)
    })
  }
}

/**
 * The color of the first automation that moves a target, as a CSS color,
 * or undefined when it has none. A control shows it as a small dot.
 */
export function useAutomationMarker(
  target: AutomationTarget
): string | undefined {
  const key = targetKey(target)
  return useProjectStore((state) => {
    for (const automation of state.project.automations) {
      if (targetKey(automation.target) === key) {
        return colorToCss(automation.color)
      }
    }
    return undefined
  })
}

/** What a control bound to an automatable target is handed. */
export type AutomationBinding = {
  /** Entries for the control's right-click menu. */
  items: ContextItem[]
  /** The value automation is giving the control, for its `live` prop. */
  live: LiveValueFeed
  /** The dot that says the control has an automation, for its `marker` prop. */
  marker: string | undefined
}

/**
 * Everything a knob or fader needs to take part in automation: its menu
 * entries, the live value and the marker. `target` may be a new object on
 * every render; the binding only changes when it names something else.
 */
export function useAutomation(target: AutomationTarget): AutomationBinding {
  const key = targetKey(target)
  // The effect of an effect target can move to another track, and then the
  // target has to name that track for the command to find it.
  const track = "track" in target ? target.track : -1
  const marker = useAutomationMarker(target)
  const stable = useMemo(
    () => ({ items: automationItems(target), live: automationFeed(target) }),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the key and the track are the target
    [key, track]
  )
  return { ...stable, marker }
}

/** The automation of each setting of an effect or instrument editor. */
export type ParamAutomation = {
  contextItems(info: ParamInfo, index: number): readonly ContextItem[]
  live(info: ParamInfo, index: number): LiveValueFeed
  marker(info: ParamInfo, index: number): string | undefined
}

/**
 * The three options `useParamBinding` takes to make every control of an
 * editor automatable. `targetOf` names the target of the setting at an
 * index, and `key` is what that depends on: the effect or the channel.
 */
export function useParamAutomation(
  targetOf: (index: number) => AutomationTarget,
  key: string
): ParamAutomation {
  const automations = useProjectStore((state) => state.project.automations)
  const stable = useMemo(() => {
    const items = new Map<number, ContextItem[]>()
    const feeds = new Map<number, LiveValueFeed>()
    return {
      contextItems(_info: ParamInfo, index: number) {
        let found = items.get(index)
        if (!found) {
          found = automationItems(targetOf(index))
          items.set(index, found)
        }
        return found
      },
      live(_info: ParamInfo, index: number) {
        let found = feeds.get(index)
        if (!found) {
          found = automationFeed(targetOf(index))
          feeds.set(index, found)
        }
        return found
      },
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `key` stands for what `targetOf` closes over
  }, [key])
  const markers = useMemo(() => {
    const byTarget = new Map<string, string>()
    for (const automation of automations) {
      const target = targetKey(automation.target)
      if (!byTarget.has(target)) {
        byTarget.set(target, colorToCss(automation.color))
      }
    }
    return byTarget
  }, [automations])
  return {
    ...stable,
    marker: (_info, index) => markers.get(targetKey(targetOf(index))),
  }
}
