import type { ReactNode, Ref } from "react"
import {
  ArrowRightDoubleIcon,
  Cursor01Icon,
  Eraser01Icon,
  FitToScreenIcon,
  GhostIcon,
  PaintBrush01Icon,
  PencilEdit02Icon,
  ZoomInAreaIcon,
} from "@hugeicons/core-free-icons"
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react"

import type { ChannelId } from "@/bindings"
import { ActionButton } from "@/components/action-button"
import { NumberField } from "@/components/audio/number-field"
import { ContextActions } from "@/components/context-actions"
import { Button } from "@/components/ui/button"
import { Kbd } from "@/components/ui/kbd"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  isChecked,
  runAction,
  useAction,
  useActionEnabled,
  useAppSelector,
  useShortcutLabel,
} from "@/lib/actions"
import { useHint } from "@/lib/store/hint"
import { useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { colorToCss } from "@/lib/units"
import { isNoteArticulation, NOTE_ARTICULATIONS } from "@/lib/note-expression"

import { useSession } from "./context"
import { VIEW_MENU } from "./menu"
import { isSnapId, SNAP_OPTIONS, type SnapOption } from "./snap"
import { usePianoRollStore } from "./store"
import { ScaleControls } from "./scale-controls"
import { StampMenu } from "./stamp-menu"
import { NoteColorPicker } from "./note-color-picker"
import { WaveformHelperControl } from "./waveform-helper"

type RollButtonProps = {
  action: string
  icon: IconSvgElement
}

/**
 * A toolbar button for a piano roll action. Like the shell's action button,
 * but it lights up while its action is on and hands the keyboard back to
 * the notes after a click.
 */
function RollButton({ action: id, icon }: RollButtonProps) {
  const session = useSession()
  const action = useAction(id)
  const enabled = useActionEnabled(id)
  const checked = useAppSelector((state) =>
    action ? isChecked(action, state) : false
  )
  const shortcut = useShortcutLabel(id)
  const title = action?.title ?? id
  const hint = useHint(shortcut ? `${title} (${shortcut})` : title)

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            variant={checked ? "secondary" : "ghost"}
            size="icon-sm"
            aria-label={title}
            aria-pressed={action?.checked ? checked : undefined}
            disabled={!enabled}
            className={checked ? "text-brand" : undefined}
            onClick={() => {
              void runAction(id)
              session.focusGrid()
            }}
            {...hint}
          />
        }
      >
        <HugeiconsIcon icon={icon} strokeWidth={2} />
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {title}
        {shortcut && <Kbd>{shortcut}</Kbd>}
      </TooltipContent>
    </Tooltip>
  )
}

function Group({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div role="group" aria-label={label} className="flex items-center gap-0.5">
      {children}
    </div>
  )
}

function ChannelPicker({ channelId }: { channelId: ChannelId }) {
  const session = useSession()
  const channels = useProjectStore((state) => state.project.channels)
  const selectChannel = useUiStore((state) => state.selectChannel)
  const hint = useHint("The channel whose notes are being edited")
  const items = channels.map((channel) => ({
    value: channel.id,
    label: channel.name,
  }))

  return (
    <Select
      items={items}
      value={channelId}
      onValueChange={(next: ChannelId | null) => {
        if (next !== null) selectChannel(next)
        session.focusGrid()
      }}
    >
      <SelectTrigger
        size="sm"
        aria-label="Channel"
        className="max-w-44 min-w-28"
        {...hint}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent alignItemWithTrigger={false} align="start">
        <SelectGroup>
          {channels.map((channel) => (
            <SelectItem key={channel.id} value={channel.id}>
              <span
                aria-hidden
                className="size-2 shrink-0 rounded-[2px]"
                style={{ background: colorToCss(channel.color) }}
              />
              <span className="truncate">{channel.name}</span>
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}

const SNAP_GROUPS: SnapOption["group"][] = ["none", "step", "beat", "bar"]

function SnapPicker() {
  const session = useSession()
  const snap = usePianoRollStore((state) => state.snap)
  const setSnap = usePianoRollStore((state) => state.setSnap)
  const hint = useHint(
    "Snap: the grid that drawing, moving and resizing notes stick to. Hold Alt while dragging to ignore it"
  )
  const items = SNAP_OPTIONS.map((option) => ({
    value: option.id,
    label: option.label,
  }))

  return (
    <label className="flex items-center gap-1.5">
      <span className="text-muted-foreground">Snap</span>
      <Select
        items={items}
        value={snap}
        onValueChange={(next: string | null) => {
          if (isSnapId(next)) setSnap(next)
          session.focusGrid()
        }}
      >
        <SelectTrigger size="sm" aria-label="Snap" className="w-24" {...hint}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent alignItemWithTrigger={false} align="start">
          {SNAP_GROUPS.map((group, index) => (
            <SelectGroup key={group}>
              {index > 0 && <SelectSeparator />}
              {SNAP_OPTIONS.filter((option) => option.group === group).map(
                (option) => (
                  <SelectItem key={option.id} value={option.id}>
                    {option.label}
                  </SelectItem>
                )
              )}
            </SelectGroup>
          ))}
        </SelectContent>
      </Select>
    </label>
  )
}

type ToolbarProps = {
  channelId: ChannelId
  /** Filled by the panel with the position under the pointer. */
  readoutRef: Ref<HTMLOutputElement>
}

function ArticulationPicker() {
  const session = useSession()
  const articulation = usePianoRollStore((state) => state.drawArticulation)
  const glideTicks = usePianoRollStore((state) => state.drawGlideTicks)
  const setArticulation = usePianoRollStore((state) => state.setDrawArticulation)
  const setGlideTicks = usePianoRollStore((state) => state.setDrawGlideTicks)
  return (
    <Group label="New note articulation">
      <Select items={NOTE_ARTICULATIONS} value={articulation} onValueChange={(next) => {
        if (isNoteArticulation(next)) setArticulation(next)
        session.focusGrid()
      }}>
        <SelectTrigger size="sm" aria-label="New note articulation" className="w-36">
          <SelectValue />
        </SelectTrigger>
        <SelectContent alignItemWithTrigger={false} align="start">
          <SelectGroup>
            {NOTE_ARTICULATIONS.map((item) => <SelectItem key={item.value} value={item.value}>{item.label}</SelectItem>)}
          </SelectGroup>
        </SelectContent>
      </Select>
      {articulation === "portamento" && <NumberField
        size="sm" aria-label="Portamento duration" value={glideTicks}
        onValueChange={setGlideTicks} min={1} max={245760} step={1}
        coarseStep={60} unit="ticks" className="w-24"
      />}
    </Group>
  )
}

function DrawColorPicker() {
  const session = useSession()
  const color = usePianoRollStore((state) => state.drawColorGroup)
  const setColor = usePianoRollStore((state) => state.setDrawColorGroup)
  return <NoteColorPicker compact value={color} onChange={(value) => {
    if (value !== "keep") setColor(value)
    session.focusGrid()
  }} />
}

/** The strip above the grid: channel, tools, snap, view switches, readout. */
export function PianoRollToolbar({ channelId, readoutRef }: ToolbarProps) {
  const readoutHint = useHint(
    "Position under the pointer as bar, beat and tick, and the key"
  )

  return (
    <ContextActions items={VIEW_MENU}>
      <div
        role="toolbar"
        aria-label="Piano roll"
        className="flex h-9 shrink-0 items-center gap-3 overflow-x-auto overflow-y-hidden border-b bg-chassis/40 px-1.5 whitespace-nowrap"
      >
        <ChannelPicker channelId={channelId} />
        <Group label="Tools">
          <RollButton action="pianoRoll.toolDraw" icon={PencilEdit02Icon} />
          <RollButton action="pianoRoll.toolPaint" icon={PaintBrush01Icon} />
          <RollButton action="pianoRoll.toolSelect" icon={Cursor01Icon} />
          <RollButton action="pianoRoll.toolErase" icon={Eraser01Icon} />
        </Group>
        <SnapPicker />
        <ArticulationPicker />
        <DrawColorPicker />
        <ScaleControls />
        <StampMenu />
        <WaveformHelperControl />
        <ActionButton action="pianoRoll.quantize" variant="outline" size="sm">
          Note tools
        </ActionButton>
        <Group label="View">
          <RollButton action="pianoRoll.ghosts" icon={GhostIcon} />
          <ActionButton action="pianoRoll.editGhosts" variant="ghost" size="sm">Edit ghosts</ActionButton>
          <RollButton action="pianoRoll.follow" icon={ArrowRightDoubleIcon} />
          <RollButton action="pianoRoll.zoomFit" icon={FitToScreenIcon} />
          <RollButton action="pianoRoll.zoomSelection" icon={ZoomInAreaIcon} />
        </Group>
        <output
          ref={readoutRef}
          aria-label="Position under the pointer"
          className="ml-auto flex h-6 min-w-36 shrink-0 items-center justify-end rounded-sm bg-display px-2 font-readout text-[0.6875rem] whitespace-pre text-display-foreground empty:before:text-display-dim empty:before:content-['–.–.–––']"
          {...readoutHint}
        />
      </div>
    </ContextActions>
  )
}
