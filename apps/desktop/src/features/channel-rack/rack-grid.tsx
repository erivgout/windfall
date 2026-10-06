import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent,
} from "react"

import type { ChannelId } from "@/bindings"
import { ActionButton } from "@/components/action-button"
import { StepGridGroup, type StepGridGroupHandle } from "@/components/audio"
import { hasSampleDrag, readSampleDrag } from "@/lib/dnd"
import { useProjectStore } from "@/lib/store/project"
import { usePlayhead } from "@/lib/store/realtime"
import { useChannelIds, useSelectedPatternId } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"
import { clamp, DEFAULT_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

import {
  addChannelFromFile,
  moveChannelTo,
  replaceSampleFromFile,
} from "./channel-ops"
import { ChannelRow } from "./channel-row"
import {
  fitPitch,
  LEFT_COLUMNS,
  LEFT_WIDTH,
  MIN_STEP_PITCH,
  PITCH_VAR,
  pitches,
  ROW_HEIGHT,
  RULER_HEIGHT,
  STEPS_INSET,
  STEPS_TRAIL,
} from "./layout"
import { StepRuler } from "./step-ruler"
import { moveIndex, stepsPerBeat } from "./steps"

/** MIME type of a rack row being dragged to a new position. */
export const CHANNEL_DRAG_TYPE = "application/x-windfall-channel"

type Drop =
  /** Into the gap above row `index`; the row count means below the last. */
  | { kind: "insert"; index: number }
  /** Onto a channel's button, to give it the dragged sample. */
  | { kind: "replace"; channel: ChannelId }

function sameDrop(a: Drop | null, b: Drop | null): boolean {
  if (a === null || b === null) return a === b
  if (a.kind === "insert") return b.kind === "insert" && a.index === b.index
  return b.kind === "replace" && a.channel === b.channel
}

/**
 * The rows of the rack under a ruler. The left columns stay put while the
 * steps scroll sideways, and the ruler stays put while the rows scroll down.
 */
export function RackGrid() {
  const ids = useChannelIds()
  const pattern = useSelectedPatternId()
  const lengthSteps = useProjectStore(
    (state) =>
      state.project.patterns.find((item) => item.id === pattern)?.lengthSteps ??
      DEFAULT_PATTERN_STEPS
  )
  const signature = useProjectStore(
    (state) => state.project.settings.timeSignature
  )
  const anySolo = useProjectStore((state) =>
    state.project.channels.some((channel) => channel.solo)
  )

  const scroller = useRef<HTMLDivElement>(null)
  const group = useRef<StepGridGroupHandle>(null)
  const rows = useRef<HTMLDivElement>(null)
  const caret = useRef<HTMLDivElement>(null)
  const shownStep = useRef<number | null>(null)
  const dragged = useRef<ChannelId | null>(null)
  const [drop, setDrop] = useState<Drop | null>(null)
  const [viewWidth, setViewWidth] = useState(0)
  const pitch =
    viewWidth > 0 ? fitPitch(viewWidth, lengthSteps) : MIN_STEP_PITCH

  useEffect(() => {
    const element = scroller.current
    if (!element) return
    const observer = new ResizeObserver(() => setViewWidth(element.clientWidth))
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  // The playhead is drawn straight into the page: one attribute per row when
  // the step changes and one transform per frame, with no render.
  usePlayhead((tick, playing) => {
    const transport = useTransportStore.getState()
    const running =
      playing && transport.mode === "pattern" && transport.pattern === pattern
    const position = tick / TICKS_PER_STEP
    const step = running && position < lengthSteps ? Math.floor(position) : null
    if (step !== shownStep.current) {
      shownStep.current = step
      group.current?.setPlayStep(step)
    }
    const mark = caret.current
    if (!mark) return
    if (step === null) {
      mark.style.visibility = "hidden"
    } else {
      mark.style.visibility = "visible"
      mark.style.transform = `translateX(${position * pitch}px)`
    }
  })

  function gapAt(event: DragEvent): number {
    const top = rows.current?.getBoundingClientRect().top ?? 0
    return clamp(Math.round((event.clientY - top) / ROW_HEIGHT), 0, ids.length)
  }

  /** Where the thing being dragged would land, or null when nowhere. */
  function dropAt(event: DragEvent): Drop | null {
    if (hasSampleDrag(event)) {
      const button =
        event.target instanceof Element
          ? event.target.closest<HTMLElement>("[data-channel-button]")
          : null
      const channel = Number(button?.dataset.channelButton)
      if (button && Number.isInteger(channel)) {
        return { kind: "replace", channel }
      }
      return { kind: "insert", index: gapAt(event) }
    }
    if (event.dataTransfer.types.includes(CHANNEL_DRAG_TYPE)) {
      const from = ids.indexOf(dragged.current ?? -1)
      const index = gapAt(event)
      // A gap next to the row itself would leave it where it is.
      if (from >= 0 && moveIndex(from, index, ids.length) === null) return null
      return { kind: "insert", index }
    }
    return null
  }

  function accepts(event: DragEvent): boolean {
    return (
      hasSampleDrag(event) ||
      event.dataTransfer.types.includes(CHANNEL_DRAG_TYPE)
    )
  }

  function onDragStart(event: DragEvent<HTMLDivElement>) {
    const grip =
      event.target instanceof Element
        ? event.target.closest<HTMLElement>("[data-drag-channel]")
        : null
    if (!grip) return
    const id = Number(grip.dataset.dragChannel)
    dragged.current = id
    event.dataTransfer.setData(CHANNEL_DRAG_TYPE, String(id))
    event.dataTransfer.effectAllowed = "move"
    const row = grip.closest("[data-channel-row]")?.firstElementChild
    if (row) event.dataTransfer.setDragImage(row, 8, ROW_HEIGHT / 2)
  }

  function onDragOver(event: DragEvent<HTMLDivElement>) {
    if (!accepts(event)) return
    event.preventDefault()
    event.dataTransfer.dropEffect = hasSampleDrag(event) ? "copy" : "move"
    const next = dropAt(event)
    setDrop((current) => (sameDrop(current, next) ? current : next))
  }

  function onDragLeave(event: DragEvent<HTMLDivElement>) {
    const to = event.relatedTarget
    if (to instanceof Node && event.currentTarget.contains(to)) return
    setDrop(null)
  }

  function onDrop(event: DragEvent<HTMLDivElement>) {
    if (!accepts(event)) return
    event.preventDefault()
    const target = dropAt(event)
    setDrop(null)
    if (target === null) return

    const sample = readSampleDrag(event)
    if (sample) {
      if (target.kind === "replace") {
        void replaceSampleFromFile(target.channel, sample.path)
      } else {
        void addChannelFromFile(sample.path, target.index)
      }
      return
    }
    const id = Number(event.dataTransfer.getData(CHANNEL_DRAG_TYPE))
    const from = ids.indexOf(id)
    if (from < 0 || target.kind !== "insert") return
    const to = moveIndex(from, target.index, ids.length)
    if (to !== null) void moveChannelTo(id, to)
  }

  const groupSize = stepsPerBeat(signature)
  const replacing = drop?.kind === "replace" ? drop.channel : null

  return (
    <div
      ref={scroller}
      data-slot="rack-scroll"
      className="relative min-h-0 flex-1 overflow-auto overscroll-contain"
      style={{ [PITCH_VAR]: `${pitch}px` } as CSSProperties}
      onDragStart={onDragStart}
      onDragEnd={() => {
        dragged.current = null
        setDrop(null)
      }}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      <div
        className="flex min-h-full min-w-full flex-col"
        style={{
          width: pitches(lengthSteps, LEFT_WIDTH + STEPS_INSET + STEPS_TRAIL),
        }}
      >
        <div
          className="sticky top-0 z-20 flex shrink-0 border-b bg-background"
          style={{ height: RULER_HEIGHT }}
        >
          <div
            className={`${LEFT_COLUMNS} sticky left-0 z-10 shrink-0 bg-background text-[0.625rem] text-muted-foreground`}
            style={{ width: LEFT_WIDTH }}
          >
            <span className="col-span-2" />
            <span className="text-center">Pan</span>
            <span className="text-center">Vol</span>
            <span className="pl-3.5">Channel</span>
            <span className="text-right">Mixer</span>
          </div>
          <div style={{ paddingLeft: STEPS_INSET }}>
            <StepRuler
              lengthSteps={lengthSteps}
              signature={signature}
              caretRef={caret}
            />
          </div>
        </div>

        <StepGridGroup
          ref={group}
          role="group"
          aria-label="Channels"
          className="relative gap-0"
        >
          <div ref={rows}>
            {pattern !== null &&
              ids.map((id) => (
                <ChannelRow
                  key={id}
                  id={id}
                  pattern={pattern}
                  lengthSteps={lengthSteps}
                  groupSize={groupSize}
                  anySolo={anySolo}
                  dropTarget={replacing === id}
                />
              ))}
          </div>
          {drop?.kind === "insert" && (
            <div
              aria-hidden
              data-slot="rack-drop-line"
              data-index={drop.index}
              className="pointer-events-none absolute inset-x-0 z-20 h-0.5 -translate-y-px bg-brand shadow-[0_0_6px_var(--wf-brand)]"
              style={{ top: drop.index * ROW_HEIGHT }}
            />
          )}
        </StepGridGroup>

        <div className="flex min-h-9 flex-1 items-start">
          <div
            className="sticky left-0 flex items-center gap-2 px-1.5 py-1.5"
            style={{ minWidth: LEFT_WIDTH }}
          >
            <ActionButton action="channel.add" variant="ghost" size="sm" />
            <span className="text-muted-foreground">
              or drag a sample here from the browser
            </span>
          </div>
        </div>
      </div>
    </div>
  )
}
