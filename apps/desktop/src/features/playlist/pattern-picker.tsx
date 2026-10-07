import { Add01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { memo, type KeyboardEvent } from "react"

import type { PatternId } from "@/bindings"
import { ActionButton } from "@/components/action-button"
import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { runAction } from "@/lib/actions"
import {
  usePattern,
  usePatternIds,
  useSelectedPatternId,
  useSettings,
} from "@/lib/store/selectors"
import { setTransportPattern } from "@/lib/store/transport"
import { ticksPerBar } from "@/lib/time"
import { colorToCss, TICKS_PER_STEP } from "@/lib/units"
import { cn } from "@/lib/utils"

import { PICKER_WIDTH } from "./layout"
import { useClipCounts } from "./selectors"
import { useLiveHint } from "./use-live-hint"

/** The row's menu acts on the selected pattern; a right press selects the row first. */
const PATTERN_MENU: ContextItem[] = [
  "pattern.rename",
  "pattern.duplicate",
  contextSeparator,
  "pattern.delete",
  contextSeparator,
  "pattern.add",
]

/** "2 bars", or the steps when the pattern is not a whole number of bars. */
export function describePatternLength(
  lengthSteps: number,
  barTicks: number
): string {
  const bars = (lengthSteps * TICKS_PER_STEP) / barTicks
  if (Number.isInteger(bars)) return bars === 1 ? "1 bar" : `${bars} bars`
  return lengthSteps === 1 ? "1 step" : `${lengthSteps} steps`
}

type PatternRowProps = {
  id: PatternId
  selected: boolean
  clips: number
  barTicks: number
}

const PatternRow = memo(function PatternRow({
  id,
  selected,
  clips,
  barTicks,
}: PatternRowProps) {
  const pattern = usePattern(id)
  const name = pattern?.name ?? ""
  const used =
    clips === 0
      ? "not on the timeline yet"
      : `on the timeline ${clips === 1 ? "once" : `${clips} times`}`
  const hint = useLiveHint(
    selected
      ? `${name} is the pattern Draw and Paint place. It is ${used}. Double-click to rename`
      : `Click to place ${name} with Draw and Paint. It is ${used}`
  )
  if (!pattern) return null

  return (
    <ContextActions items={PATTERN_MENU}>
      <button
        type="button"
        aria-pressed={selected}
        data-pattern={id}
        onPointerDown={(event) => {
          // The menu's actions work on the selected pattern.
          if (event.button === 2 && !selected) void setTransportPattern(id)
        }}
        onClick={() => void setTransportPattern(id)}
        onDoubleClick={() => void runAction("pattern.rename")}
        className={cn(
          "group flex h-7 w-full shrink-0 items-center gap-2 border-l-2 border-transparent pr-2 pl-1.5 text-left outline-none hover:bg-accent/60 focus-visible:bg-accent",
          selected && "border-brand bg-accent text-accent-foreground"
        )}
        {...hint}
      >
        <span
          aria-hidden
          className="h-3.5 w-2 shrink-0 rounded-[2px]"
          style={{ backgroundColor: colorToCss(pattern.color) }}
        />
        <span
          className={cn(
            "min-w-0 flex-1 truncate",
            selected ? "font-medium" : "text-foreground/85"
          )}
        >
          {pattern.name}
        </span>
        <span className="shrink-0 font-readout text-[0.625rem] text-muted-foreground">
          {describePatternLength(pattern.lengthSteps, barTicks)}
        </span>
      </button>
    </ContextActions>
  )
})

/**
 * The patterns of the project, at the left of the timeline. The selected
 * one is the brush: Draw and Paint place it. Selecting here selects the
 * pattern everywhere, so the channel rack shows it too.
 */
export function PatternPicker() {
  const ids = usePatternIds()
  const selected = useSelectedPatternId()
  const counts = useClipCounts(ids)
  const barTicks = ticksPerBar(useSettings().timeSignature)

  function onKeyDown(event: KeyboardEvent) {
    const step =
      event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0
    if (step === 0 || event.ctrlKey || event.metaKey || event.altKey) return
    // Kept from the playlist's own arrow keys, which move clips.
    event.preventDefault()
    void runAction(step === 1 ? "pattern.next" : "pattern.previous")
  }

  return (
    <aside
      aria-label="Patterns"
      className="flex shrink-0 flex-col border-r bg-chassis/30"
      style={{ width: PICKER_WIDTH }}
    >
      <div className="flex h-6 shrink-0 items-center border-b pr-0.5 pl-2 text-muted-foreground">
        <span className="flex-1 text-[0.6875rem] font-medium">Patterns</span>
        <ActionButton
          action="pattern.add"
          variant="ghost"
          size="icon-xs"
          tooltipSide="right"
        >
          <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
        </ActionButton>
      </div>
      <div
        role="group"
        aria-label="Pattern to place"
        onKeyDown={onKeyDown}
        className="flex min-h-0 flex-1 flex-col overflow-y-auto py-0.5"
      >
        {ids.map((id, index) => (
          <PatternRow
            key={id}
            id={id}
            selected={id === selected}
            clips={counts[index] ?? 0}
            barTicks={barTicks}
          />
        ))}
      </div>
    </aside>
  )
}
