import { useRef } from "react"
import { ArrowDown01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useHint } from "@/lib/store/hint"

import { useSession } from "./context"
import { LANE_KINDS, type LaneKind } from "./lane-math"
import { MAX_LANE_HEIGHT, MIN_LANE_HEIGHT, usePianoRollStore } from "./store"

const KEY_STEP_PX = 12

/** Picks what the lane under the grid shows: velocity or pan. */
export function LaneHeader() {
  const session = useSession()
  const kind = usePianoRollStore((state) => state.laneKind)
  const setKind = usePianoRollStore((state) => state.setLaneKind)
  const hint = useHint("What the bars under the notes show and edit")
  const label = LANE_KINDS.find((item) => item.id === kind)?.label ?? kind

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        aria-label={`Lane shows ${label.toLowerCase()}`}
        className="flex h-6 w-full items-center justify-between gap-0.5 px-1 text-[0.625rem] text-muted-foreground outline-none hover:bg-foreground/5 hover:text-foreground focus-visible:bg-foreground/10 focus-visible:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        {...hint}
      >
        <span className="truncate">{label}</span>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          strokeWidth={2}
          className="size-3 shrink-0"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="min-w-32">
        <DropdownMenuRadioGroup
          value={kind}
          onValueChange={(value: LaneKind) => {
            setKind(value)
            session.focusGrid()
          }}
        >
          {LANE_KINDS.map((item) => (
            <DropdownMenuRadioItem key={item.id} value={item.id} closeOnClick>
              {item.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

/** The bar between the grid and the lane. Drag it to give the lane more room. */
export function LaneResizer() {
  const height = usePianoRollStore((state) => state.laneHeight)
  const setHeight = usePianoRollStore((state) => state.setLaneHeight)
  const drag = useRef<{ y: number; height: number } | null>(null)
  const hint = useHint("Drag to resize the lane under the notes")

  return (
    <div
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize the lane under the notes"
      aria-valuemin={MIN_LANE_HEIGHT}
      aria-valuemax={MAX_LANE_HEIGHT}
      aria-valuenow={height}
      tabIndex={0}
      className="h-[5px] cursor-ns-resize touch-none border-y bg-chassis outline-none hover:bg-brand/50 focus-visible:bg-brand/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset active:bg-brand/70"
      onPointerDown={(event) => {
        if (event.button !== 0) return
        drag.current = { y: event.clientY, height }
        event.currentTarget.setPointerCapture(event.pointerId)
      }}
      onPointerMove={(event) => {
        const start = drag.current
        if (start) setHeight(start.height + start.y - event.clientY)
      }}
      onPointerUp={() => {
        drag.current = null
      }}
      onPointerCancel={() => {
        drag.current = null
      }}
      onKeyDown={(event) => {
        if (event.key === "ArrowUp") setHeight(height + KEY_STEP_PX)
        else if (event.key === "ArrowDown") setHeight(height - KEY_STEP_PX)
        else return
        event.preventDefault()
      }}
      {...hint}
    />
  )
}
