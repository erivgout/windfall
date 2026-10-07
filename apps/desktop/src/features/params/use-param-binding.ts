import { useLayoutEffect, useRef, useState } from "react"

import type { ParamInfo } from "@/bindings"
import type { LiveValueFeed } from "@/components/audio"
import type { ContextItem } from "@/components/context-actions"
import { newGestureId } from "@/lib/store/gesture"

import { clampParam, readParam } from "./access"
import { paramIndex, type ParamDescriptor } from "./descriptors"

/**
 * Sends one setting to wherever the settings live: `index` is the setting's
 * place in the descriptor, and `gesture` is the same for every change of one
 * drag. Return the promise of the dispatch, so the control lets go of its
 * own value only once the stored one has caught up.
 */
export type SetParam = (
  index: number,
  value: number,
  gesture?: number
) => unknown

/** The props `bind(id)` hands to a `ParamControl`. */
export type ParamBinding = {
  info: ParamInfo
  value: number
  disabled: boolean
  onValueChange(value: number): void
  onGestureStart(): void
  onGestureEnd(): void
  /**
   * What the setting offers in its control's right-click menu besides what
   * every value control does: the entries of `contextItems` below.
   */
  contextItems: readonly ContextItem[]
  /** What `live` below gave for this setting. */
  live?: LiveValueFeed
  /** What `marker` below gave for this setting. */
  marker?: string
}

/** Several settings moved by one control, as one undo step. */
export type ParamGroupBinding = {
  set(id: string, value: number): void
  onGestureStart(): void
  onGestureEnd(): void
}

export type ParamBinder = {
  /** The props for the `ParamControl` of the setting with this id. */
  (id: string): ParamBinding
  /** The description of a setting. */
  info(id: string): ParamInfo
  /** The value a setting shows now: the dragged one, or the stored one. */
  value(id: string): number
  /**
   * For a control that moves more than one setting, such as an envelope
   * editor. Everything set between its gesture start and end is one undo
   * step. `key` tells such controls apart.
   */
  group(key: string): ParamGroupBinding
  /** True when there are no settings to edit. */
  disabled: boolean
}

export type ParamBindingOptions<Params extends object> = {
  descriptor: ParamDescriptor<Params>
  /** The stored settings. Without them every control is disabled. */
  params: Params | null | undefined
  setParam: SetParam
  /**
   * Entries for the right-click menu of one setting's control, about the
   * setting itself: `index` is its place in the descriptor, the number the
   * `set…Param` commands know it by. This is where an entry that makes an
   * automation clip for the setting goes. Called when a control renders, so
   * keep it cheap and return the same array for the same setting.
   */
  contextItems?: (info: ParamInfo, index: number) => readonly ContextItem[]
  /**
   * A feed of the value something else is giving a setting right now, as
   * automation does while the song plays. The setting's knob shows it,
   * without rendering. Return the same feed for the same setting.
   */
  live?: (info: ParamInfo, index: number) => LiveValueFeed | undefined
  /**
   * The CSS color of the dot that says a setting has an automation, or
   * undefined when it has none.
   */
  marker?: (info: ParamInfo, index: number) => string | undefined
}

const NO_ITEMS: readonly ContextItem[] = []

type Run = { gesture: number; touched: Set<number>; last: unknown }

type Handlers = Pick<
  ParamBinding,
  "onValueChange" | "onGestureStart" | "onGestureEnd"
>

const NOTHING_HELD: ReadonlyMap<number, number> = new Map()

/**
 * Connects the controls of an editor to its settings:
 *
 *     const bind = useParamBinding({ descriptor, params, setParam })
 *     <ParamControl {...bind("filter.cutoffHz")} />
 *
 * A control shows its own value while it is moved, and every change of one
 * drag goes to `setParam` under one gesture id, so a drag is one undo step.
 * The handlers of a setting stay the same from render to render.
 */
export function useParamBinding<Params extends object>(
  options: ParamBindingOptions<Params>
): ParamBinder {
  const { descriptor, params } = options
  // What the controls being moved show, by setting index.
  const [held, setHeld] = useState(NOTHING_HELD)
  const latest = useRef(options)
  useLayoutEffect(() => {
    latest.current = options
  })

  const [core] = useState(() => {
    const runs = new Map<string, Run>()
    const holders = new Map<number, number>()
    const handlers = new Map<string, Handlers>()
    const groups = new Map<string, ParamGroupBinding>()

    function begin(key: string) {
      runs.set(key, {
        gesture: newGestureId(),
        touched: new Set(),
        last: undefined,
      })
    }

    function set(key: string, id: string, value: number) {
      const { descriptor: current, setParam } = latest.current
      const index = paramIndex(current, id)
      const next = clampParam(current.params[index], value)
      const run = runs.get(key)
      if (!run) {
        // Outside a gesture a change is an undo step of its own.
        void setParam(index, next)
        return
      }
      if (!run.touched.has(index)) {
        run.touched.add(index)
        holders.set(index, (holders.get(index) ?? 0) + 1)
      }
      setHeld((map) => new Map(map).set(index, next))
      run.last = setParam(index, next, run.gesture)
    }

    function end(key: string) {
      const run = runs.get(key)
      if (!run) return
      runs.delete(key)
      const release = () => {
        const free: number[] = []
        for (const index of run.touched) {
          const count = (holders.get(index) ?? 1) - 1
          if (count > 0) {
            holders.set(index, count)
          } else {
            holders.delete(index)
            free.push(index)
          }
        }
        if (free.length === 0) return
        setHeld((map) => {
          const next = new Map(map)
          for (const index of free) next.delete(index)
          return next.size === 0 ? NOTHING_HELD : next
        })
      }
      // The dragged value stays until the stored one has caught up, so the
      // control does not jump back for a frame.
      void Promise.resolve(run.last).then(release, release)
    }

    return {
      handlers(id: string): Handlers {
        let found = handlers.get(id)
        if (!found) {
          const key = `param:${id}`
          found = {
            onGestureStart: () => begin(key),
            onValueChange: (value) => set(key, id, value),
            onGestureEnd: () => end(key),
          }
          handlers.set(id, found)
        }
        return found
      },
      group(name: string): ParamGroupBinding {
        let found = groups.get(name)
        if (!found) {
          const key = `group:${name}`
          found = {
            onGestureStart: () => begin(key),
            set: (id, value) => set(key, id, value),
            onGestureEnd: () => end(key),
          }
          groups.set(name, found)
        }
        return found
      },
    }
  })

  const disabled = params === null || params === undefined

  function info(id: string): ParamInfo {
    return descriptor.params[paramIndex(descriptor, id)]
  }

  function value(id: string): number {
    const index = paramIndex(descriptor, id)
    const dragged = held.get(index)
    if (dragged !== undefined) return dragged
    const described = descriptor.params[index]
    return disabled ? described.default : readParam(params, described)
  }

  function bind(id: string): ParamBinding {
    const index = paramIndex(descriptor, id)
    const described = descriptor.params[index]
    return {
      info: described,
      value: value(id),
      disabled,
      contextItems: options.contextItems?.(described, index) ?? NO_ITEMS,
      live: options.live?.(described, index),
      marker: options.marker?.(described, index),
      ...core.handlers(id),
    }
  }

  return Object.assign(bind, { info, value, group: core.group, disabled })
}
