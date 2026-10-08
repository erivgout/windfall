import { create } from "zustand"
import { persist } from "zustand/middleware"

import { onProjectReplaced } from "@/lib/store/replaced"
import type { NoteArticulation } from "@/bindings"
import { isNoteArticulation } from "@/lib/note-expression"
import { isNoteColorGroup } from "@/lib/note-colors"
import { DEFAULT_VELOCITY, TICKS_PER_STEP } from "@/lib/units"

import { LANE_KINDS, type LaneKind } from "./lane-math"
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
  editGhosts: boolean
  /** Scroll along with the playhead. */
  follow: boolean
  laneKind: LaneKind
  laneHeight: number
  /** Length and velocity the next drawn note gets: those of the last one touched. */
  lastLength: number
  lastVelocity: number
  drawArticulation: NoteArticulation
  drawGlideTicks: number
  drawColorGroup: number | null
  /** Mirrors of editor state, so buttons and menus can follow them. */
  selectionCount: number
  clipboardCount: number

  setTool(tool: Tool): void
  setDrawArticulation(articulation: NoteArticulation): void
  setDrawGlideTicks(ticks: number): void
  setDrawColorGroup(group: number | null): void
  setSnap(snap: SnapId): void
  setScaleRoot(root: number): void
  setScaleId(id: ScaleId): void
  setHighlightScale(enabled: boolean): void
  setSnapToScale(enabled: boolean): void
  setGhosts(ghosts: boolean): void
  setEditGhosts(enabled: boolean): void
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
      editGhosts: false,
      follow: false,
      laneKind: "velocity",
      laneHeight: DEFAULT_LANE_HEIGHT,
      lastLength: TICKS_PER_STEP,
      lastVelocity: DEFAULT_VELOCITY,
      drawArticulation: "normal",
      drawGlideTicks: TICKS_PER_STEP,
      drawColorGroup: null,
      selectionCount: 0,
      clipboardCount: 0,

      setTool: (tool) => set({ tool }),
      setDrawArticulation: (drawArticulation) => {
        if (isNoteArticulation(drawArticulation)) set({ drawArticulation })
      },
      setDrawGlideTicks: (ticks) => {
        if (Number.isFinite(ticks)) set({ drawGlideTicks: Math.round(Math.min(245760, Math.max(1, ticks))) })
      },
      setDrawColorGroup: (drawColorGroup) => {
        if (drawColorGroup === null || isNoteColorGroup(drawColorGroup)) set({ drawColorGroup })
      },
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
      setEditGhosts: (editGhosts) => set((state) => ({ editGhosts, ghosts: editGhosts || state.ghosts })),
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
        editGhosts: state.editGhosts,
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
          editGhosts: typeof stored.editGhosts === "boolean" ? stored.editGhosts : current.editGhosts,
          follow:
            typeof stored.follow === "boolean" ? stored.follow : current.follow,
          laneKind:
            LANE_KINDS.some((kind) => kind.id === stored.laneKind)
              ? stored.laneKind!
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
    drawArticulation: "normal",
    drawGlideTicks: TICKS_PER_STEP,
    drawColorGroup: null,
  })
)
