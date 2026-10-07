import type { HitPart } from "@/lib/canvas"

export type Tool = "draw" | "paint" | "select" | "erase" | "mute"

export const TOOLS: readonly Tool[] = [
  "draw",
  "paint",
  "select",
  "erase",
  "mute",
]

/** What was under the pointer when a button went down. */
export type Press = {
  tool: Tool
  /** 0 left, 1 middle, 2 right, as on a pointer event. */
  button: number
  /** Ctrl, or Cmd on macOS. */
  mod: boolean
  shift: boolean
  /** The clip under the pointer and which part of it, or null on empty grid. */
  hit: { id: number; part: HitPart } | null
}

/** What a press starts. The pointer handling carries it out. */
export type Intent =
  | { kind: "none" }
  | { kind: "pan" }
  /** Drag a box around clips. `additive` keeps what was already selected. */
  | { kind: "marquee"; additive: boolean }
  | { kind: "move"; id: number; additive: boolean }
  | { kind: "trim-start"; id: number }
  | { kind: "resize-end"; id: number }
  /** Put one clip of the selected pattern where the button is released. */
  | { kind: "place" }
  /** Lay clips of the selected pattern back to back along the stroke. */
  | { kind: "paint" }
  /** Delete every clip the stroke crosses. */
  | { kind: "erase" }
  /** Mute or unmute every clip the stroke crosses. */
  | { kind: "mute" }
  /** Open the right-click menu for the selected clips or the empty grid. */
  | { kind: "menu"; id: number | null }

function onClip(hit: { id: number; part: HitPart }, additive: boolean): Intent {
  switch (hit.part) {
    case "start-edge":
      return { kind: "trim-start", id: hit.id }
    case "end-edge":
      return { kind: "resize-end", id: hit.id }
    case "body":
      return { kind: "move", id: hit.id, additive }
    default: {
      const _exhaustive: never = hit.part
      return _exhaustive
    }
  }
}

/**
 * Decides what a press does from the tool, the button, the modifier keys
 * and what is under the pointer.
 *
 * - Middle button pans.
 * - Ctrl+drag selects with a box in every tool.
 * - Right button deletes, except in the Select tool where it opens a menu.
 * - Draw, Paint and Select all move and resize the clips they press on.
 */
export function intentFor(press: Press): Intent {
  const { tool, button, mod, shift, hit } = press
  if (button === 1) return { kind: "pan" }
  if (button === 2) {
    return tool === "select"
      ? { kind: "menu", id: hit?.id ?? null }
      : { kind: "erase" }
  }
  if (button !== 0) return { kind: "none" }
  if (mod) return { kind: "marquee", additive: shift }

  switch (tool) {
    case "draw":
      return hit ? onClip(hit, shift) : { kind: "place" }
    case "paint":
      return hit ? onClip(hit, shift) : { kind: "paint" }
    case "select":
      return hit ? onClip(hit, shift) : { kind: "marquee", additive: shift }
    case "erase":
      return { kind: "erase" }
    case "mute":
      return { kind: "mute" }
    default: {
      const _exhaustive: never = tool
      return _exhaustive
    }
  }
}

/** The mouse cursor for a tool over a part of a clip, or over empty grid. */
export function cursorFor(tool: Tool, part: HitPart | null): string {
  if (tool === "erase") return part ? "not-allowed" : "default"
  if (tool === "mute") return part ? "pointer" : "default"
  if (part === "start-edge" || part === "end-edge") return "ew-resize"
  if (part === "body") return "grab"
  return tool === "select" ? "default" : "crosshair"
}
