import { create } from "zustand"
import { persist } from "zustand/middleware"

import { onProjectReplaced } from "@/lib/store/replaced"
import { DEFAULT_VELOCITY, TICKS_PER_STEP } from "@/lib/units"

import type { LaneKind } from "./lane-math"
import { DEFAULT_SNAP, isSnapId, type SnapId } from "./snap"
import { isScaleId, isScaleRoot, type ScaleId } from "./scales"

export type Tool = "draw" | "paint" | "select" | "erase"

export const TOOLS: Tool[] = ["draw", "paint", "select", "erase"]

export const MIN_LANE_HEIGHT = 44
export const MAX_LANE_HEIGHT = 260
export const DEFAULT_LANE_HEIGHT = 84

type PianoRollState = {
  tool: Tool
  snap: SnapId
  scaleRoot: number
  scaleId: ScaleId
  highlightScale: boolean
  snapToScale: boolean
  /** Show the other channels' notes behind this one's. */
  ghosts: boolean
  /** Scroll along with the playhead. */
  follow: boolean
  laneKind: LaneKind
  laneHeight: number
  /** Length and velocity the next drawn note gets: those of the last one touched. */
  lastLength: number
  lastVelocity: number
  /** Mirrors of editor state, so buttons and menus can follow them. */
  selectionCount: number
  clipboardCount: number

  setTool(tool: Tool): void
  setSnap(snap: SnapId): void
  setScaleRoot(root: number): void
  setScaleId(id: ScaleId): void
  setHighlightScale(enabled: boolean): void
  setSnapToScale(enabled: boolean): void
  setGhosts(ghosts: boolean): void
  setFollow(follow: boolean): void
  setLaneKind(kind: LaneKind): void
  setLaneHeight(height: number): void
  rememberNote(length: number, velocity: number): void
}

/**
 * How the piano roll is set up. It outlives the panel, which is unmounted
 * whenever another editor tab is showing. The tool choices that are a
 * matter of taste are remembered across restarts.
 */
export const usePianoRollStore = create<PianoRollState>()(
  persist(
    (set) => ({
      tool: "draw",
      snap: DEFAULT_SNAP,
      scaleRoot: 0,
      scaleId: "major",
      highlightScale: false,
      snapToScale: false,
      ghosts: true,
      follow: false,
      laneKind: "velocity",
      laneHeight: DEFAULT_LANE_HEIGHT,
      lastLength: TICKS_PER_STEP,
      lastVelocity: DEFAULT_VELOCITY,
      selectionCount: 0,
      clipboardCount: 0,

      setTool: (tool) => set({ tool }),
      setSnap: (snap) => set({ snap }),
      setScaleRoot: (scaleRoot) => {
        if (isScaleRoot(scaleRoot)) set({ scaleRoot })
      },
      setScaleId: (scaleId) => {
        if (isScaleId(scaleId)) set({ scaleId })
      },
      setHighlightScale: (highlightScale) => set({ highlightScale }),
      setSnapToScale: (snapToScale) => set({ snapToScale }),
      setGhosts: (ghosts) => set({ ghosts }),
      setFollow: (follow) => set({ follow }),
      setLaneKind: (laneKind) => set({ laneKind }),
      setLaneHeight: (height) =>
        set({
          laneHeight: Math.round(
            Math.min(MAX_LANE_HEIGHT, Math.max(MIN_LANE_HEIGHT, height))
          ),
        }),
      rememberNote: (length, velocity) =>
        set({
          lastLength: Math.max(1, Math.round(length)),
          lastVelocity: Math.min(1, Math.max(0, velocity)),
        }),
    }),
    {
      name: "windfall.pianoRoll",
      version: 1,
      partialize: (state) => ({
        snap: state.snap,
        scaleRoot: state.scaleRoot,
        scaleId: state.scaleId,
        highlightScale: state.highlightScale,
        snapToScale: state.snapToScale,
        ghosts: state.ghosts,
        follow: state.follow,
        laneKind: state.laneKind,
        laneHeight: state.laneHeight,
      }),
      merge: (saved, current) => {
        const stored = (
          saved && typeof saved === "object" ? saved : {}
        ) as Partial<PianoRollState>
        return {
          ...current,
          snap: isSnapId(stored.snap) ? stored.snap : current.snap,
          scaleRoot: isScaleRoot(stored.scaleRoot)
            ? stored.scaleRoot
            : current.scaleRoot,
          scaleId: isScaleId(stored.scaleId) ? stored.scaleId : current.scaleId,
          highlightScale:
            typeof stored.highlightScale === "boolean"
              ? stored.highlightScale
              : current.highlightScale,
          snapToScale:
            typeof stored.snapToScale === "boolean"
              ? stored.snapToScale
              : current.snapToScale,
          ghosts:
            typeof stored.ghosts === "boolean" ? stored.ghosts : current.ghosts,
          follow:
            typeof stored.follow === "boolean" ? stored.follow : current.follow,
          laneKind:
            stored.laneKind === "velocity" || stored.laneKind === "pan"
              ? stored.laneKind
              : current.laneKind,
          laneHeight:
            typeof stored.laneHeight === "number" &&
            Number.isFinite(stored.laneHeight)
              ? Math.round(
                  Math.min(
                    MAX_LANE_HEIGHT,
                    Math.max(MIN_LANE_HEIGHT, stored.laneHeight)
                  )
                )
              : current.laneHeight,
        }
      },
    }
  )
)

// The Draw tool gives a new note the length and velocity of the last one
// touched. Those belong to the song that note was in: the first note of a
// new project came out four bars long, and grew its pattern to match.
onProjectReplaced(() =>
  usePianoRollStore.setState({
    lastLength: TICKS_PER_STEP,
    lastVelocity: DEFAULT_VELOCITY,
  })
)
