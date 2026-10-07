import { useState, type DragEvent } from "react"

import type { EffectId, TrackId } from "@/bindings"

import {
  canDrop,
  chainLength,
  dropEffect,
  findEffect,
  moveTarget,
} from "./effect-ops"

/*
 * Dragging an effect: within its chain to reorder it, onto another strip
 * to move it there, and with Ctrl or Alt held to leave it and drop a copy.
 * Strips and the inspector are both places to drag from and drop on.
 */

/** MIME type of an effect dragged out of a slot. */
export const EFFECT_DRAG_TYPE = "application/x-windfall-effect"

// A drag's data cannot be read until the drop, and the drop target has to
// know what is coming to show where it would land.
let dragged: EffectId | null = null

export function startEffectDrag(event: DragEvent, effect: EffectId) {
  dragged = effect
  event.dataTransfer.setData(EFFECT_DRAG_TYPE, String(effect))
  event.dataTransfer.effectAllowed = "copyMove"
}

export function endEffectDrag() {
  dragged = null
}

function hasEffectDrag(event: DragEvent): boolean {
  return event.dataTransfer.types.includes(EFFECT_DRAG_TYPE)
}

function readEffectDrag(event: DragEvent): EffectId | null {
  const raw = event.dataTransfer.getData(EFFECT_DRAG_TYPE)
  const id = raw === "" ? NaN : Number(raw)
  return Number.isInteger(id) ? id : dragged
}

/** Ctrl or Alt held during a drag asks for a copy. */
function wantsCopy(event: DragEvent): boolean {
  return event.ctrlKey || event.altKey
}

/**
 * The gap of the chain the pointer is over: 0 above the first effect, and
 * one more for every effect whose middle the pointer is below. A zone
 * with no rows in it, such as a strip that shows only a badge, gives the
 * gap after the last effect.
 */
export function gapAt(zone: Element, clientY: number, count: number): number {
  const rows = zone.querySelectorAll("[data-effect-row]")
  if (rows.length === 0) return count
  let gap = 0
  for (const row of rows) {
    const rect = row.getBoundingClientRect()
    if (clientY > rect.top + rect.height / 2) gap += 1
  }
  return gap
}

export type EffectDrop = {
  /** The gap a dragged effect would land in, or null while none would. */
  gap: number | null
  zone: {
    onDragOver(event: DragEvent<HTMLElement>): void
    onDragLeave(event: DragEvent<HTMLElement>): void
    onDrop(event: DragEvent<HTMLElement>): void
  }
}

/**
 * Makes an element a place to drop effects into `track`'s chain. Spread
 * `zone` on the element and draw the insertion mark at `gap`.
 */
export function useEffectDrop(track: TrackId): EffectDrop {
  const [gap, setGap] = useState<number | null>(null)

  /** Where the drag would land, or null when it cannot or would not move. */
  function landing(event: DragEvent<HTMLElement>, effect: EffectId | null) {
    const found = findEffect(effect)
    const copy = wantsCopy(event)
    if (!found || effect === null || !canDrop(effect, track, copy)) return null
    const at = gapAt(event.currentTarget, event.clientY, chainLength(track))
    if (!copy && moveTarget(found, track, at) === null) return null
    return { gap: at, copy }
  }

  return {
    gap,
    zone: {
      onDragOver(event) {
        if (!hasEffectDrag(event)) return
        const target = landing(event, dragged)
        if (target === null) {
          setGap(null)
          return
        }
        event.preventDefault()
        event.dataTransfer.dropEffect = target.copy ? "copy" : "move"
        setGap((current) => (current === target.gap ? current : target.gap))
      },
      onDragLeave(event) {
        const to = event.relatedTarget
        if (to instanceof Node && event.currentTarget.contains(to)) return
        setGap(null)
      },
      onDrop(event) {
        if (!hasEffectDrag(event)) return
        const effect = readEffectDrag(event)
        const target = landing(event, effect)
        setGap(null)
        endEffectDrag()
        if (effect === null || target === null) return
        event.preventDefault()
        void dropEffect(effect, track, target.gap, target.copy)
      },
    },
  }
}
