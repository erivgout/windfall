import { memo, type KeyboardEvent, type PointerEvent } from "react"

import type { ChannelId } from "@/bindings"
import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import {
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover"
import { useHint } from "@/lib/store/hint"
import { colorToCss } from "@/lib/units"
import { cn } from "@/lib/utils"

import { FILL_INTERVALS } from "./actions"
import { useAudition } from "./audition"
import {
  selectChannel,
  setChannelColor,
  toggleMute,
  toggleSolo,
} from "./channel-ops"
import { ColorSwatches } from "./color-swatches"
import { useRackStore } from "./rack-store"

type ChannelButtonProps = {
  id: ChannelId
  name: string
  color: number
  selected: boolean
  muted: boolean
  solo: boolean
  /** Muted, or silent because another channel is soloed. */
  dimmed: boolean
  hasSample: boolean
  /** A dragged sample is over the button and would replace its sample. */
  dropTarget: boolean
}

/** Finds the control to move to with the arrow keys, in this row or the next. */
function neighbor(
  from: HTMLElement,
  selector: string,
  rows: -1 | 0 | 1
): HTMLElement | null {
  let row = from.closest<HTMLElement>("[data-channel-row]")
  if (rows !== 0) {
    const next =
      rows < 0 ? row?.previousElementSibling : row?.nextElementSibling
    row = next instanceof HTMLElement && next.dataset.channelRow ? next : null
  }
  return row?.querySelector<HTMLElement>(selector) ?? null
}

function ColorChip({
  id,
  name,
  color,
}: Pick<ChannelButtonProps, "id" | "name" | "color">) {
  const open = useRackStore((state) => state.colorPickerFor === id)
  const openColorPicker = useRackStore((state) => state.openColorPicker)
  const hint = useHint("Channel color: click to choose another")

  return (
    <Popover
      open={open}
      onOpenChange={(next) => openColorPicker(next ? id : null)}
    >
      <PopoverTrigger
        render={
          <button
            type="button"
            aria-label={`${name} color`}
            className="h-full w-2 shrink-0 rounded-l-[3px] outline-none hover:brightness-125 focus-visible:ring-2 focus-visible:ring-ring"
            style={{ backgroundColor: colorToCss(color) }}
            {...hint}
          />
        }
      />
      <PopoverContent align="start" className="w-auto gap-2 p-2">
        <PopoverTitle className="text-xs font-medium">
          Color of {name}
        </PopoverTitle>
        <ColorSwatches
          value={color}
          onPick={(picked) => {
            void setChannelColor(id, picked)
            openColorPicker(null)
          }}
        />
      </PopoverContent>
    </Popover>
  )
}

/**
 * The name plate of a row. Pressing it plays the channel, clicking it
 * selects the channel and opens its settings, and right-clicking lists what
 * can be done to the channel.
 */
export const ChannelButton = memo(function ChannelButton({
  id,
  name,
  color,
  selected,
  muted,
  solo,
  dimmed,
  hasSample,
  dropTarget,
}: ChannelButtonProps) {
  const audition = useAudition(id)
  const hint = useHint(
    hasSample
      ? `${name}: press to hear it, click to open its settings, right-click for more. Drop a sample here to replace its sound`
      : `${name} has no sample yet. Drop one here from the browser, or click to open its settings`
  )

  // Mute and solo are worded for this row. Everything else is a registry
  // action on the selected channel, which a right-click makes this one.
  const items: ContextItem[] = [
    "channel.rename",
    "channel.color",
    "channel.duplicate",
    contextSeparator,
    { title: muted ? "Unmute" : "Mute", run: () => toggleMute(id) },
    { title: solo ? "Unsolo" : "Solo", run: () => toggleSolo(id) },
    contextSeparator,
    "channel.clearSteps",
    ...FILL_INTERVALS.map((every) => `channel.fill${every}`),
    "channel.shiftLeft",
    "channel.shiftRight",
    contextSeparator,
    "channel.moveUp",
    "channel.moveDown",
    contextSeparator,
    "channel.routeToNewTrack",
    "channel.showInMixer",
    contextSeparator,
    "channel.delete",
  ]

  function onPointerDown(event: PointerEvent<HTMLButtonElement>) {
    if (event.button === 0) audition.start()
  }

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return
    let target: HTMLElement | null
    if (event.key === "ArrowUp") {
      target = neighbor(event.currentTarget, "[data-channel-button]", -1)
    } else if (event.key === "ArrowDown") {
      target = neighbor(event.currentTarget, "[data-channel-button]", 1)
    } else if (event.key === "ArrowRight") {
      target = neighbor(
        event.currentTarget,
        '[data-slot="step-grid"] button[tabindex="0"]',
        0
      )
    } else {
      return
    }
    event.preventDefault()
    target?.focus()
  }

  return (
    <div
      className={cn(
        "flex h-5.5 min-w-0 rounded-[3px] bg-(--wf-step-off) ring-1 ring-transparent",
        selected && "bg-(--wf-step-off-alt) ring-foreground/45",
        !hasSample &&
          "bg-transparent outline-1 -outline-offset-1 outline-foreground/30 outline-dashed",
        dropTarget && "ring-2 ring-brand"
      )}
    >
      <ColorChip id={id} name={name} color={color} />
      <ContextActions items={items}>
        <button
          type="button"
          data-channel-button={id}
          aria-pressed={selected}
          aria-label={hasSample ? name : `${name}, no sample`}
          onClick={() => selectChannel(id, { openSettings: true })}
          onContextMenu={() => selectChannel(id)}
          onPointerDown={onPointerDown}
          onPointerUp={audition.stop}
          onPointerLeave={(event) => {
            audition.stop()
            hint.onPointerLeave()
            event.currentTarget.removeAttribute("title")
          }}
          onPointerCancel={audition.stop}
          onPointerEnter={(event) => {
            hint.onPointerEnter()
            // A name that does not fit gets the browser's tooltip.
            const label = event.currentTarget.firstElementChild
            if (label && label.scrollWidth > label.clientWidth) {
              event.currentTarget.title = name
            }
          }}
          onFocus={hint.onFocus}
          onBlur={() => {
            audition.stop()
            hint.onBlur()
          }}
          onKeyDown={onKeyDown}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-1.5 rounded-r-[3px] pr-1.5 pl-1.5 text-left outline-none hover:bg-foreground/8 focus-visible:ring-2 focus-visible:ring-ring active:bg-foreground/12",
            (dimmed || !hasSample) && "text-muted-foreground"
          )}
        >
          <span className="min-w-0 flex-1 truncate font-medium">{name}</span>
          {dropTarget ? (
            <span className="shrink-0 text-[0.625rem] text-brand">replace</span>
          ) : (
            !hasSample && (
              <span className="shrink-0 text-[0.625rem] text-warn">
                no sample
              </span>
            )
          )}
        </button>
      </ContextActions>
    </div>
  )
})
