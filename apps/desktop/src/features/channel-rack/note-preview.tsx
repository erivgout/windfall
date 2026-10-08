import {
  memo,
  useEffect,
  useMemo,
  useRef,
  type KeyboardEvent,
  type ReactNode,
} from "react"

import type { ChannelId, Note, PatternId } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { runAction } from "@/lib/actions"
import { useHint } from "@/lib/store/hint"
import { useProjectStore } from "@/lib/store/project"
import { usePlayhead } from "@/lib/store/realtime"
import {
  getProjectGeneration,
  useProjectGeneration,
} from "@/lib/store/replaced"
import { selectedPatternId } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { TICKS_PER_STEP } from "@/lib/units"
import { cn } from "@/lib/utils"

import { pitches } from "./layout"
import {
  needsNotePreview,
  notePreviewGeometry,
  previewSteps,
} from "./note-preview-geometry"
import { useRackStore, type RackNoteView } from "./rack-store"

type Target = { pattern: PatternId; channel: ChannelId; generation: number }

function current(target: Target): boolean {
  const { project } = useProjectStore.getState()
  return (
    target.generation === getProjectGeneration() &&
    selectedPatternId(project, useTransportStore.getState().pattern) ===
      target.pattern &&
    project.channels.some((item) => item.id === target.channel)
  )
}

/** No async transport change: only the lane currently shown may navigate. */
function open(target: Target) {
  if (!current(target)) return
  useUiStore.getState().selectChannel(target.channel)
  void runAction("view.pianoRoll")
}

type PreviewProps = {
  target: Target
  name: string
  notes: readonly Note[] | undefined
  lengthSteps: number
  color: string
  quiet: boolean
}

const NotePreview = memo(function NotePreview({
  target,
  name,
  notes,
  lengthSteps,
  color,
  quiet,
}: PreviewProps) {
  const geometry = useMemo(
    () => notePreviewGeometry(notes, lengthSteps),
    [notes, lengthSteps]
  )
  const pressed = useRef<Target | null>(null)
  const caret = useRef<HTMLSpanElement>(null)
  const mounted = useRef(false)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])
  const hint = useHint(
    "Note preview in rack. Click or press Enter to open this channel in the piano roll. Space plays or stops. The row view menu offers step buttons"
  )
  // Cursor style writes per frame, independent of the number of notes; no React.
  usePlayhead((tick, playing) => {
    const mark = caret.current
    if (!mark) return
    const transport = useTransportStore.getState()
    const inside =
      Number.isFinite(tick) &&
      tick >= 0 &&
      tick < previewSteps(lengthSteps) * TICKS_PER_STEP
    const running =
      playing &&
      transport.playing &&
      transport.mode === "pattern" &&
      transport.pattern === target.pattern &&
      target.generation === getProjectGeneration() &&
      inside
    mark.style.visibility = running ? "visible" : "hidden"
    if (running) mark.style.left = pitches(tick / TICKS_PER_STEP)
  })

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey ||
      event.nativeEvent.isComposing
    )
      return
    if (event.key === "Enter") {
      event.preventDefault()
      event.stopPropagation()
      if (!event.repeat && mounted.current) open(target)
      return
    }
    let row = event.currentTarget.closest<HTMLElement>("[data-channel-row]")
    let next: HTMLElement | null
    if (event.key === "ArrowLeft")
      next = row?.querySelector("[data-channel-button]") ?? null
    else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      const neighbor =
        event.key === "ArrowUp"
          ? row?.previousElementSibling
          : row?.nextElementSibling
      row = neighbor instanceof HTMLElement ? neighbor : null
      next =
        row?.querySelector('[data-slot="step-grid"] button[tabindex="0"]') ??
        null
    } else return
    event.preventDefault()
    event.stopPropagation()
    next?.focus({ preventScroll: true })
  }

  const description = `${geometry.count} ${geometry.count === 1 ? "note" : "notes"} in the pattern${geometry.outside ? `; ${geometry.outside} outside the preview` : ""}${geometry.dense ? "; dense notes are combined in the thumbnail" : ""}`
  return (
    // Keep the existing channel-name arrow-navigation hook, without step buttons.
    <div
      data-slot="step-grid"
      className="h-5.5 shrink-0"
      style={{ width: pitches(previewSteps(lengthSteps)) }}
    >
      <Button
        variant="ghost"
        className={cn(
          "relative h-full w-full overflow-hidden rounded-[3px] border-0 p-0",
          quiet && "opacity-40"
        )}
        style={{ backgroundColor: "var(--wf-step-off)" }}
        tabIndex={0}
        data-slot="rack-note-preview"
        aria-label={`${name} note preview. Open in piano roll`}
        title={description}
        onKeyDown={onKeyDown}
        onPointerDown={(event) => {
          pressed.current = event.button === 0 ? target : null
        }}
        onPointerCancel={() => {
          pressed.current = null
        }}
        onClick={(event) => {
          const captured = event.detail > 0 ? pressed.current : target
          pressed.current = null
          if (mounted.current && captured) open(captured)
        }}
        {...hint}
      >
        <svg
          aria-hidden
          viewBox={`0 0 ${geometry.width} ${geometry.height}`}
          preserveAspectRatio="none"
          style={{ width: "100%", height: "100%" }}
        >
          <path
            data-note-shapes
            d={geometry.path}
            fill={color}
            fillRule="nonzero"
          />
        </svg>
        <span className="sr-only">{description}</span>
        <span
          ref={caret}
          aria-hidden
          data-slot="rack-note-playhead"
          className="pointer-events-none absolute inset-y-0 w-px bg-(--wf-playhead)"
          style={{ visibility: "hidden" }}
        />
        {geometry.count === 0 && (
          <span className="absolute inset-0 flex items-center justify-center text-muted-foreground">
            No notes in this pattern
          </span>
        )}
      </Button>
    </div>
  )
})

/** The view choice belongs to this document/lane, not to its notes. */
export function RackNoteArea({
  pattern,
  channel,
  name,
  notes,
  lengthSteps,
  color,
  quiet,
  children,
}: Omit<PreviewProps, "target"> & {
  pattern: PatternId
  channel: ChannelId
  children: ReactNode
}) {
  const generation = useProjectGeneration()
  const key = `${pattern}:${channel}`
  const choice = useRackStore((state) => state.noteViews[key] ?? "auto")
  const automatic = useMemo(() => needsNotePreview(notes), [notes])
  const showNotes = choice === "notes" || (choice === "auto" && automatic)
  const target = useMemo(
    () => ({ pattern, channel, generation }),
    [pattern, channel, generation]
  )
  const focus = useRef<HTMLDivElement>(null)

  function choose(view: RackNoteView) {
    if (!current(target)) return
    useRackStore.getState().setNoteView(key, view)
  }

  // Menu finalFocus follows the new control when changing view unmounts the old.
  return (
    <>
      <div ref={focus} className="contents">
        {showNotes ? (
          <NotePreview
            key={`${generation}:${key}`}
            target={target}
            name={name}
            notes={notes}
            lengthSteps={lengthSteps}
            color={color}
            quiet={quiet}
          />
        ) : (
          children
        )}
      </div>
      <DropdownMenu key={`${generation}:${key}`}>
        <DropdownMenuTrigger
          render={
            <Button
              variant="ghost"
              size="icon-xs"
              className="sticky right-0 z-10 -mr-3 h-5.5 w-3 rounded-none bg-background"
              aria-label={`${name} row view`}
            />
          }
        >
          <span aria-hidden>⋮</span>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="end"
          finalFocus={() =>
            focus.current?.querySelector<HTMLElement>(
              '[data-slot="step-grid"] button[tabindex="0"]'
            ) ?? false
          }
        >
          <DropdownMenuGroup>
            <DropdownMenuLabel>Row view</DropdownMenuLabel>
            {(
              [
                ["auto", "Automatic steps or notes"],
                ["steps", "Show steps"],
                ["notes", "Show notes"],
              ] as const
            ).map(([view, label]) => (
              <DropdownMenuItem
                key={view}
                onClick={() => choose(view)}
                aria-label={`${label}${choice === view ? ", current view" : ""}`}
              >
                {label}
                {choice === view && (
                  <span aria-hidden className="ml-auto">
                    ✓
                  </span>
                )}
              </DropdownMenuItem>
            ))}
            <DropdownMenuItem onClick={() => open(target)}>
              Open in piano roll
            </DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    </>
  )
}
