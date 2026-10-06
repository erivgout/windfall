// SPDX-License-Identifier: MIT
import * as React from "react"

import { cn } from "@/lib/utils"

import { formatMs, formatPercent } from "./units"

export type EnvelopeValues = {
  attackMs: number
  decayMs: number
  /** Sustain level, 0 to 1. */
  sustain: number
  releaseMs: number
}

export type EnvelopeLimits = {
  maxAttackMs: number
  maxDecayMs: number
  maxReleaseMs: number
}

export const DEFAULT_ENVELOPE_LIMITS: EnvelopeLimits = {
  maxAttackMs: 5000,
  maxDecayMs: 5000,
  maxReleaseMs: 10000,
}

type TimeField = "attackMs" | "decayMs" | "releaseMs"

function clamp(value: number, min: number, max: number): number {
  return Number.isNaN(value) ? min : Math.min(max, Math.max(min, value))
}

/** Clamps every value to its range. Times snap to 0.1 ms, sustain to 0.001. */
export function constrainEnvelope(
  values: EnvelopeValues,
  limits: EnvelopeLimits = DEFAULT_ENVELOPE_LIMITS
): EnvelopeValues {
  const time = (value: number, max: number) =>
    Math.round(clamp(value, 0, max) * 10) / 10
  return {
    attackMs: time(values.attackMs, limits.maxAttackMs),
    decayMs: time(values.decayMs, limits.maxDecayMs),
    sustain: Math.round(clamp(values.sustain, 0, 1) * 1000) / 1000,
    releaseMs: time(values.releaseMs, limits.maxReleaseMs),
  }
}

/** The fields of `next` that differ from `current`. */
export function envelopePatch(
  current: EnvelopeValues,
  next: EnvelopeValues
): Partial<EnvelopeValues> {
  const patch: Partial<EnvelopeValues> = {}
  for (const field of [
    "attackMs",
    "decayMs",
    "sustain",
    "releaseMs",
  ] as const) {
    if (next[field] !== current[field]) {
      patch[field] = next[field]
    }
  }
  return patch
}

// The sustain stage has no length of its own; it is drawn this wide.
const HOLD_SHARE = 0.2
const MIN_SPAN_MS = 100

/**
 * The length of the time axis for an envelope: the next round number (1, 2
 * or 5 times a power of ten) that leaves room for all four stages.
 */
export function envelopeSpanMs(values: EnvelopeValues): number {
  const needed = Math.max(
    MIN_SPAN_MS,
    (values.attackMs + values.decayMs + values.releaseMs) / (0.95 - HOLD_SHARE)
  )
  const magnitude = 10 ** Math.floor(Math.log10(needed))
  for (const factor of [1, 2, 5, 10]) {
    if (factor * magnitude >= needed) {
      return factor * magnitude
    }
  }
  return 10 * magnitude
}

type StepSize = "fine" | "normal" | "coarse"

/** One keyboard step for a time: a share of the value, so short times step finely. */
export function stepEnvelopeTime(
  value: number,
  direction: number,
  size: StepSize
): number {
  const step =
    size === "fine"
      ? Math.max(0.1, value * 0.005)
      : size === "coarse"
        ? Math.max(10, value * 0.25)
        : Math.max(1, value * 0.05)
  return value + direction * step
}

const SUSTAIN_STEP: Record<StepSize, number> = {
  fine: 0.001,
  normal: 0.01,
  coarse: 0.1,
}

type NodeName = "attack" | "decay" | "release"

const NODES: Record<
  NodeName,
  { label: string; time: TimeField; max: keyof EnvelopeLimits; level: boolean }
> = {
  attack: {
    label: "Attack",
    time: "attackMs",
    max: "maxAttackMs",
    level: false,
  },
  decay: {
    label: "Decay and sustain",
    time: "decayMs",
    max: "maxDecayMs",
    level: true,
  },
  release: {
    label: "Release",
    time: "releaseMs",
    max: "maxReleaseMs",
    level: false,
  },
}
const NODE_NAMES = Object.keys(NODES) as NodeName[]

type DragState = {
  pointerId: number
  node: NodeName
  x: number
  y: number
  span: number
  time: number
  level: number
}

const PAD = { top: 10, right: 12, bottom: 18, left: 12 }
const FALLBACK_SIZE = { width: 320, height: 128 }

function axisLabel(ms: number): string {
  if (ms === 0) {
    return "0"
  }
  return ms >= 1000 ? `${ms / 1000} s` : `${ms} ms`
}

function describeNode(node: NodeName, values: EnvelopeValues): string {
  const time = formatMs(values[NODES[node].time])
  return node === "decay"
    ? `${time}, sustain ${formatPercent(values.sustain)}`
    : time
}

type EnvelopeEditorProps = Omit<
  React.ComponentProps<"div">,
  "onChange" | "children"
> &
  EnvelopeValues &
  Partial<EnvelopeLimits> & {
    /** Called with only the fields that changed. Always inside a gesture. */
    onChange?: (patch: Partial<EnvelopeValues>) => void
    onGestureStart?: () => void
    onGestureEnd?: () => void
    /** Restored for a node by double-click and Ctrl/Cmd-click. */
    defaults?: Partial<EnvelopeValues>
    /** Curve color as any CSS color. Defaults to `--wf-brand`. */
    color?: string
    disabled?: boolean
  }

/**
 * An attack, decay, sustain, release editor on a time axis. Drag a node, or
 * focus it and use the arrow keys: left and right change its time, up and
 * down the sustain level.
 */
function EnvelopeEditor({
  className,
  style,
  attackMs,
  decayMs,
  sustain,
  releaseMs,
  maxAttackMs = DEFAULT_ENVELOPE_LIMITS.maxAttackMs,
  maxDecayMs = DEFAULT_ENVELOPE_LIMITS.maxDecayMs,
  maxReleaseMs = DEFAULT_ENVELOPE_LIMITS.maxReleaseMs,
  onChange,
  onGestureStart,
  onGestureEnd,
  defaults,
  color,
  disabled = false,
  ...props
}: EnvelopeEditorProps) {
  const limits: EnvelopeLimits = { maxAttackMs, maxDecayMs, maxReleaseMs }
  const values = constrainEnvelope(
    { attackMs, decayMs, sustain, releaseMs },
    limits
  )

  const rootRef = React.useRef<HTMLDivElement>(null)
  const [size, setSize] = React.useState(FALLBACK_SIZE)
  React.useEffect(() => {
    const root = rootRef.current
    if (!root || typeof ResizeObserver === "undefined") {
      return undefined
    }
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect
      if (width > 0 && height > 0) {
        setSize({ width, height })
      }
    })
    observer.observe(root)
    return () => observer.disconnect()
  }, [])

  // While a node is dragged the axis keeps its length, so the node stays
  // under the pointer. It fits itself again when the drag ends.
  const [heldSpan, setHeldSpan] = React.useState<number | null>(null)
  const [shown, setShown] = React.useState<NodeName | null>(null)
  const span = heldSpan ?? envelopeSpanMs(values)

  const drag = React.useRef<DragState | null>(null)
  const gestureOpen = React.useRef(false)
  // The values last handed to the app; ahead of the props until they return.
  const current = React.useRef(values)
  React.useLayoutEffect(() => {
    current.current = values
  })

  const plotWidth = Math.max(1, size.width - PAD.left - PAD.right)
  const plotHeight = Math.max(1, size.height - PAD.top - PAD.bottom)
  const xOf = (ms: number) => PAD.left + (ms / span) * plotWidth
  const yOf = (level: number) => PAD.top + (1 - level) * plotHeight
  const hold = span * HOLD_SHARE

  const peak = { x: xOf(values.attackMs), y: yOf(1) }
  const settle = {
    x: xOf(values.attackMs + values.decayMs),
    y: yOf(values.sustain),
  }
  const letGo = { x: settle.x + xOf(hold) - xOf(0), y: settle.y }
  const finish = { x: letGo.x + xOf(values.releaseMs) - xOf(0), y: yOf(0) }
  const origin = { x: xOf(0), y: yOf(0) }
  const points: Record<NodeName, { x: number; y: number }> = {
    attack: peak,
    decay: settle,
    release: finish,
  }

  function emit(next: EnvelopeValues) {
    const patch = envelopePatch(
      current.current,
      constrainEnvelope(next, limits)
    )
    if (Object.keys(patch).length === 0) {
      return
    }
    if (!gestureOpen.current) {
      gestureOpen.current = true
      onGestureStart?.()
    }
    current.current = { ...current.current, ...patch }
    onChange?.(patch)
  }

  function endGesture() {
    if (gestureOpen.current) {
      gestureOpen.current = false
      onGestureEnd?.()
    }
  }

  function resetNode(node: NodeName) {
    if (!defaults) {
      return
    }
    const { time, level } = NODES[node]
    const next = { ...current.current }
    if (defaults[time] !== undefined) {
      next[time] = defaults[time]
    }
    if (level && defaults.sustain !== undefined) {
      next.sustain = defaults.sustain
    }
    emit(next)
    endGesture()
  }

  function handlePointerDown(
    node: NodeName,
    event: React.PointerEvent<HTMLDivElement>
  ) {
    if (disabled || event.button !== 0 || drag.current) {
      return
    }
    if (event.ctrlKey || event.metaKey) {
      resetNode(node)
      return
    }
    try {
      event.currentTarget.setPointerCapture(event.pointerId)
    } catch {
      // Without capture the drag still works over the node.
    }
    drag.current = {
      pointerId: event.pointerId,
      node,
      x: event.clientX,
      y: event.clientY,
      span,
      time: current.current[NODES[node].time],
      level: current.current.sustain,
    }
    setHeldSpan(span)
    setShown(node)
  }

  function handlePointerMove(event: React.PointerEvent<HTMLDivElement>) {
    const state = drag.current
    if (!state || state.pointerId !== event.pointerId) {
      return
    }
    const speed = event.shiftKey ? 0.1 : 1
    state.time += ((event.clientX - state.x) / plotWidth) * state.span * speed
    state.level -= ((event.clientY - state.y) / plotHeight) * speed
    state.x = event.clientX
    state.y = event.clientY
    const { time, level } = NODES[state.node]
    const next: EnvelopeValues = { ...current.current, [time]: state.time }
    if (level) {
      next.sustain = state.level
    }
    emit(next)
  }

  function handlePointerEnd(event: React.PointerEvent<HTMLDivElement>) {
    if (drag.current?.pointerId !== event.pointerId) {
      return
    }
    drag.current = null
    setHeldSpan(null)
    endGesture()
  }

  function handleKeyDown(
    node: NodeName,
    event: React.KeyboardEvent<HTMLDivElement>
  ) {
    if (disabled || event.altKey || event.ctrlKey || event.metaKey) {
      return
    }
    const { time, max, level } = NODES[node]
    const stepSize: StepSize = event.shiftKey ? "fine" : "normal"
    const now = current.current
    const next = { ...now }
    const moveTime = (direction: number, by: StepSize) => {
      next[time] = stepEnvelopeTime(now[time], direction, by)
    }
    const moveLevel = (direction: number, by: StepSize) => {
      if (level) {
        next.sustain = now.sustain + direction * SUSTAIN_STEP[by]
      } else {
        moveTime(direction, by)
      }
    }
    switch (event.key) {
      case "ArrowRight":
        moveTime(1, stepSize)
        break
      case "ArrowLeft":
        moveTime(-1, stepSize)
        break
      case "ArrowUp":
        moveLevel(1, stepSize)
        break
      case "ArrowDown":
        moveLevel(-1, stepSize)
        break
      case "PageUp":
        moveTime(1, "coarse")
        break
      case "PageDown":
        moveTime(-1, "coarse")
        break
      case "Home":
        next[time] = 0
        break
      case "End":
        next[time] = limits[max]
        break
      default:
        return
    }
    event.preventDefault()
    emit(next)
  }

  const ticks = [0, 1, 2, 3, 4, 5].map((index) => (span / 5) * index)
  const curve = `M ${origin.x} ${origin.y} L ${peak.x} ${peak.y} L ${settle.x} ${settle.y}`
  const tail = `M ${letGo.x} ${letGo.y} L ${finish.x} ${finish.y}`
  const area = `M ${origin.x} ${origin.y} L ${peak.x} ${peak.y} L ${settle.x} ${settle.y} L ${letGo.x} ${letGo.y} L ${finish.x} ${finish.y} Z`

  return (
    <div
      ref={rootRef}
      data-slot="envelope-editor"
      data-disabled={disabled ? "" : undefined}
      className={cn(
        "relative h-32 w-full overflow-hidden rounded-sm bg-(--wf-meter-bg)/40 select-none data-disabled:opacity-50",
        className
      )}
      style={
        {
          "--envelope-color": color ?? "var(--wf-brand)",
          ...style,
        } as React.CSSProperties
      }
      {...props}
    >
      <svg
        width={size.width}
        height={size.height}
        aria-hidden="true"
        className="absolute inset-0 block"
      >
        {ticks.map((tick, index) => (
          <g key={index}>
            <line
              x1={Math.round(xOf(tick)) + 0.5}
              x2={Math.round(xOf(tick)) + 0.5}
              y1={PAD.top}
              y2={origin.y}
              stroke="var(--wf-grid-line)"
            />
            <text
              x={xOf(tick)}
              y={size.height - 5}
              textAnchor={
                index === 0 ? "start" : index === 5 ? "end" : "middle"
              }
              className="fill-muted-foreground text-[9px] tabular-nums"
            >
              {axisLabel(tick)}
            </text>
          </g>
        ))}
        <line
          x1={PAD.left}
          x2={size.width - PAD.right}
          y1={Math.round(origin.y) + 0.5}
          y2={Math.round(origin.y) + 0.5}
          stroke="var(--wf-grid-line-strong)"
        />
        <path d={area} fill="var(--envelope-color)" fillOpacity={0.16} />
        <path
          d={curve}
          fill="none"
          stroke="var(--envelope-color)"
          strokeWidth={1.5}
          strokeLinejoin="round"
        />
        {/* The sustain stage lasts as long as the note is held. */}
        <line
          x1={settle.x}
          y1={settle.y}
          x2={letGo.x}
          y2={letGo.y}
          stroke="var(--envelope-color)"
          strokeWidth={1.5}
          strokeDasharray="3 3"
        />
        <path
          d={tail}
          fill="none"
          stroke="var(--envelope-color)"
          strokeWidth={1.5}
        />
      </svg>
      {NODE_NAMES.map((node) => {
        const { label, time, max } = NODES[node]
        return (
          <div
            key={node}
            role="slider"
            tabIndex={disabled ? -1 : 0}
            aria-label={label}
            aria-valuemin={0}
            aria-valuemax={limits[max]}
            aria-valuenow={values[time]}
            aria-valuetext={describeNode(node, values)}
            aria-disabled={disabled || undefined}
            data-slot="envelope-node"
            data-node={node}
            className={cn(
              "group/node absolute flex size-5 -translate-x-1/2 -translate-y-1/2 touch-none items-center justify-center rounded-full outline-none",
              disabled ? "cursor-default" : "cursor-grab active:cursor-grabbing"
            )}
            style={{ left: points[node].x, top: points[node].y }}
            onPointerDown={(event) => handlePointerDown(node, event)}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerEnd}
            onPointerCancel={handlePointerEnd}
            onLostPointerCapture={handlePointerEnd}
            onDoubleClick={() => {
              if (!disabled) {
                resetNode(node)
              }
            }}
            onKeyDown={(event) => handleKeyDown(node, event)}
            onKeyUp={endGesture}
            onPointerEnter={() => setShown(node)}
            onPointerLeave={() => {
              if (!drag.current) {
                setShown(null)
              }
            }}
            onFocus={() => setShown(node)}
            onBlur={() => {
              endGesture()
              setShown(null)
            }}
          >
            <span className="block size-2.5 rounded-full border-2 border-(--envelope-color) bg-background transition-transform group-hover/node:scale-125 group-focus-visible/node:scale-125 group-focus-visible/node:ring-2 group-focus-visible/node:ring-ring" />
          </div>
        )
      })}
      {shown ? (
        <div
          data-slot="envelope-readout"
          aria-hidden="true"
          className="pointer-events-none absolute top-1 right-2 text-[10px] leading-none text-muted-foreground tabular-nums"
        >
          {NODES[shown].label.split(" ")[0]}{" "}
          <span className="text-foreground">{describeNode(shown, values)}</span>
        </div>
      ) : null}
    </div>
  )
}

export { EnvelopeEditor }
export type { EnvelopeEditorProps }
