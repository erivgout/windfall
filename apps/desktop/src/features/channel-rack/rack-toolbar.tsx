import {
  Add01Icon,
  ArrowDown01Icon,
  SidebarRight01Icon,
} from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import { ActionButton } from "@/components/action-button"
import { ActionMenuItem } from "@/components/action-menu-item"
import { Knob, NumberField, percentUnit } from "@/components/audio"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useActions, useAppState } from "@/lib/actions"
import { useHint } from "@/lib/store/hint"
import { useProjectStore } from "@/lib/store/project"
import { useSelectedPatternId } from "@/lib/store/selectors"
import { clamp, DEFAULT_PATTERN_STEPS, MAX_PATTERN_STEPS } from "@/lib/units"

import { LENGTH_PRESETS } from "./actions"
import { useRackStore } from "./rack-store"
import { clampPatternLength, describeLength } from "./steps"
import { useGestureValue } from "./use-gesture-value"
import { useLiveHint } from "./use-live-hint"

function AddChannelMenuItems() {
  const actions = useActions()
  const state = useAppState()
  const add = actions.find((action) => action.id === "channel.add")
  const browser = actions.find((action) => action.id === "view.browser")

  return (
    <>
      {add && <ActionMenuItem action={add} state={state} />}
      <DropdownMenuSeparator />
      <p className="px-2 py-1.5 text-muted-foreground">
        To add a sound from a file, drag it from the browser onto the rack. Drop
        it on a channel&apos;s name to replace that channel&apos;s sound.
      </p>
      {browser && !state.ui.panels.browser && (
        <ActionMenuItem action={browser} state={state} />
      )}
    </>
  )
}

function AddChannelMenu() {
  const hint = useHint(
    "Add a channel: an empty sampler, or drag a sample in from the browser"
  )
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="outline"
            size="sm"
            aria-label="Add channel menu"
            {...hint}
          />
        }
      >
        <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
        Add channel
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          strokeWidth={2}
          className="text-muted-foreground"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-64">
        <AddChannelMenuItems />
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function PatternLength() {
  const pattern = useSelectedPatternId()
  const lengthSteps = useProjectStore(
    (state) =>
      state.project.patterns.find((item) => item.id === pattern)?.lengthSteps ??
      DEFAULT_PATTERN_STEPS
  )
  const signature = useProjectStore(
    (state) => state.project.settings.timeSignature
  )
  const length = useGestureValue(lengthSteps, async (value, dispatch) => {
    if (pattern === null) return
    await dispatch({
      type: "updatePattern",
      id: pattern,
      patch: { lengthSteps: clampPatternLength(value) },
    })
  })
  const hint = useLiveHint(
    `Pattern length: ${length.value} steps, ${describeLength(length.value, signature)}. Drag up or down, or double-click to type`
  )

  return (
    <div
      role="group"
      aria-label="Pattern length"
      className="flex items-center gap-1"
    >
      <span className="mr-0.5 text-muted-foreground">Length</span>
      <div className="flex items-center rounded-md bg-(--wf-step-off)/60 p-px">
        {LENGTH_PRESETS.map((steps) => (
          <ActionButton
            key={steps}
            action={`pattern.length${steps}`}
            variant={steps === lengthSteps ? "secondary" : "ghost"}
            size="xs"
            aria-pressed={steps === lengthSteps}
            className="w-7 px-0 font-readout text-[0.6875rem] aria-pressed:bg-background aria-pressed:shadow-xs"
          >
            {steps}
          </ActionButton>
        ))}
      </div>
      <NumberField
        size="sm"
        aria-label="Pattern length in steps"
        min={1}
        max={MAX_PATTERN_STEPS}
        step={1}
        defaultValue={DEFAULT_PATTERN_STEPS}
        unit={<span className="text-muted-foreground">steps</span>}
        className="font-readout"
        {...length}
        {...hint}
      />
    </div>
  )
}

function Swing() {
  const stored = useProjectStore((state) => state.project.settings.swing)
  const swing = useGestureValue(stored, (value, dispatch) =>
    dispatch({
      type: "updateSettings",
      patch: { swing: clamp(value, 0, 1) },
    })
  )
  const hint = useLiveHint(
    `Swing: ${percentUnit.format(swing.value)}. Delays every second step for a shuffle feel. Double-click to turn it off`
  )

  return (
    <div className="flex items-center gap-1.5" {...hint}>
      <span className="text-muted-foreground">Swing</span>
      <Knob
        size="sm"
        aria-label="Swing"
        min={0}
        max={1}
        defaultValue={0}
        {...percentUnit}
        {...swing}
      />
      <span className="w-7 font-readout text-[0.6875rem] text-foreground/85">
        {percentUnit.format(swing.value)}
      </span>
    </div>
  )
}

function SettingsToggle() {
  const open = useRackStore((state) => state.inspectorOpen)
  return (
    <ActionButton
      action="channelRack.settings"
      variant={open ? "secondary" : "ghost"}
      size="icon-sm"
      aria-pressed={open}
      tooltipSide="left"
    >
      <HugeiconsIcon icon={SidebarRight01Icon} strokeWidth={2} />
    </ActionButton>
  )
}

/** The strip above the rows: add a channel, pattern length, swing. */
export function RackToolbar() {
  return (
    <div
      role="toolbar"
      aria-label="Channel rack"
      className="flex h-9 shrink-0 items-center gap-4 overflow-x-auto overflow-y-hidden border-b bg-chassis/40 px-1.5 whitespace-nowrap"
    >
      <AddChannelMenu />
      <PatternLength />
      <Swing />
      <div className="ml-auto flex items-center">
        <SettingsToggle />
      </div>
    </div>
  )
}
