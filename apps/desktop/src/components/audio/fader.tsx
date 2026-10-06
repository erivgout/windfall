// SPDX-License-Identifier: MIT
import * as React from "react"

import { cn } from "@/lib/utils"

import { dbToGain, formatGain, gainToDb, parseGain } from "./units"
import {
  useDragValue,
  valueToNormalized,
  type ValueControlProps,
  type ValueScale,
} from "./use-drag-value"
import { ValueInput } from "./value-input"

/** The top of the fader: linear gain 2.0, about +6 dB. */
export const FADER_MAX_GAIN = 2

// The taper is straight lines between these points on a dB scale, steeper
// towards the top so the useful range around 0 dB gets most of the travel.
// Below the last point the gain falls in a straight line to silence.
const TAPER: readonly (readonly [db: number, position: number])[] = [
  [gainToDb(FADER_MAX_GAIN), 1],
  [0, 0.78],
  [-6, 0.62],
  [-12, 0.48],
  [-24, 0.27],
  [-48, 0.07],
]
const [FLOOR_DB, FLOOR_POSITION] = TAPER[TAPER.length - 1]
const FLOOR_GAIN = dbToGain(FLOOR_DB)

/**
 * Linear gain (0 to 2.0) to fader travel (0 to 1). Silence is 0, 0 dB is
 * 0.78 and +6 dB is 1.
 */
export function gainToFaderPosition(gain: number): number {
  if (!(gain > 0)) {
    return 0
  }
  if (gain >= FADER_MAX_GAIN) {
    return 1
  }
  if (gain <= FLOOR_GAIN) {
    return (gain / FLOOR_GAIN) * FLOOR_POSITION
  }
  const db = gainToDb(gain)
  for (let index = 1; index < TAPER.length; index += 1) {
    const [lowDb, lowPosition] = TAPER[index]
    if (db >= lowDb) {
      const [highDb, highPosition] = TAPER[index - 1]
      const share = (db - lowDb) / (highDb - lowDb)
      return lowPosition + share * (highPosition - lowPosition)
    }
  }
  return FLOOR_POSITION
}

/** Fader travel (0 to 1) to linear gain (0 to 2.0). */
export function faderPositionToGain(position: number): number {
  if (!(position > 0)) {
    return 0
  }
  if (position >= 1) {
    return FADER_MAX_GAIN
  }
  if (position <= FLOOR_POSITION) {
    return (position / FLOOR_POSITION) * FLOOR_GAIN
  }
  for (let index = 1; index < TAPER.length; index += 1) {
    const [lowDb, lowPosition] = TAPER[index]
    if (position >= lowPosition) {
      const [highDb, highPosition] = TAPER[index - 1]
      const share = (position - lowPosition) / (highPosition - lowPosition)
      return dbToGain(lowDb + share * (highDb - lowDb))
    }
  }
  return FLOOR_GAIN
}

/** The dB taper as a control scale. It assumes a range of 0 to 2.0. */
export const faderTaper: ValueScale = {
  toNormalized: (value) => gainToFaderPosition(value),
  fromNormalized: (normalized) => faderPositionToGain(normalized),
}

export type FaderTick = {
  /** Where the tick sits, in the fader's own value (linear gain by default). */
  value: number
  label?: string
  /** Draw the tick stronger, as for 0 dB. */
  strong?: boolean
}

export const FADER_DB_TICKS: readonly FaderTick[] = [
  { value: FADER_MAX_GAIN, label: "+6" },
  { value: 1, label: "0", strong: true },
  { value: dbToGain(-6), label: "−6" },
  { value: dbToGain(-12), label: "−12" },
  { value: dbToGain(-24), label: "−24" },
  { value: dbToGain(-48), label: "−48" },
  { value: 0, label: "−∞" },
]

type PlacedTick = FaderTick & { position: number }

/**
 * Places the marks and drops the labels (not the lines) of marks that sit
 * closer than `gap` of the travel to a labeled neighbor. The marks at the two
 * ends keep theirs.
 */
function spreadLabels(
  marks: readonly FaderTick[],
  place: (mark: FaderTick) => number,
  gap: number
): PlacedTick[] {
  const placed = marks
    .map((mark) => ({ ...mark, position: place(mark) }))
    .sort((a, b) => a.position - b.position)
  const last = placed[placed.length - 1]
  let labeled = -Infinity
  return placed.map((mark, index) => {
    const isEnd = index === 0 || index === placed.length - 1
    const fits =
      mark.position - labeled >= gap && last.position - mark.position >= gap
    if (isEnd || fits) {
      labeled = mark.position
      return mark
    }
    return { ...mark, label: undefined }
  })
}

// Length of the cap along its travel, in pixels. The travel is inset by half
// of it at both ends so the cap never leaves the fader.
const CAP = 14
const INSET = CAP / 2

type FaderProps = Omit<
  React.ComponentProps<"div">,
  "defaultValue" | "onChange"
> &
  ValueControlProps & {
    orientation?: "vertical" | "horizontal"
    /** Scale marks. Defaults to dB marks for a gain fader; false hides them. */
    ticks?: readonly FaderTick[] | false
    label?: React.ReactNode
    /** Show the value under the fader. Double-click it to type. */
    showValue?: boolean
    /** Rendered beside the travel and aligned to it, usually a `LevelMeter`. */
    meter?: React.ReactNode
    /** Fill color as any CSS color. Defaults to `--wf-control-fill`. */
    color?: string
  }

/**
 * A level fader. With no `min`, `max` or `scale` it is a gain fader: linear
 * gain 0 to 2.0 on a dB taper, with dB marks, a dB readout and 0 dB as the
 * default. Pass a range to use it for anything else.
 */
function Fader({
  className,
  style,
  orientation = "vertical",
  ticks,
  label,
  showValue = false,
  meter,
  color,
  value,
  onValueChange,
  onGestureStart,
  onGestureEnd,
  defaultValue,
  min,
  max,
  step,
  keyStep,
  scale,
  format,
  parse,
  disabled = false,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledBy,
  ...props
}: FaderProps) {
  const isGain = min === undefined && max === undefined && scale === undefined
  const low = min ?? 0
  const high = max ?? (isGain ? FADER_MAX_GAIN : 1)
  const taper = scale ?? (isGain ? faderTaper : "linear")
  const vertical = orientation === "vertical"

  const control = useDragValue({
    value,
    onValueChange,
    onGestureStart,
    onGestureEnd,
    defaultValue: defaultValue ?? (isGain ? 1 : undefined),
    min: low,
    max: high,
    step,
    keyStep,
    scale: taper,
    format: format ?? (isGain ? formatGain : undefined),
    parse: parse ?? (isGain ? parseGain : undefined),
    disabled,
    orientation,
    trackOvershoot: true,
    dragRange: (element) =>
      (vertical ? element.clientHeight : element.clientWidth) - CAP,
  })
  const labelId = React.useId()

  const marks = spreadLabels(
    ticks === false ? [] : (ticks ?? (isGain ? FADER_DB_TICKS : [])),
    (mark) => valueToNormalized(mark.value, low, high, taper),
    vertical ? 0.05 : 0.1
  )
  const percent = `${(control.normalized * 100).toFixed(3)}%`
  const along = (position: number): React.CSSProperties =>
    vertical
      ? { bottom: `${(position * 100).toFixed(3)}%` }
      : { left: `${(position * 100).toFixed(3)}%` }

  const scaleMarks =
    marks.length > 0 ? (
      <div
        data-slot="fader-scale"
        aria-hidden="true"
        className={cn(
          "relative shrink-0 text-[9px] leading-none text-muted-foreground tabular-nums",
          vertical ? "w-6" : "h-4"
        )}
      >
        <div
          className="absolute"
          style={
            vertical ? { inset: `${INSET}px 0` } : { inset: `0 ${INSET}px` }
          }
        >
          {marks.map((mark) => (
            <div
              key={mark.value}
              className={cn(
                "absolute flex items-center gap-0.5",
                mark.strong && "text-foreground",
                vertical
                  ? "right-0 translate-y-1/2"
                  : "top-0 -translate-x-1/2 flex-col"
              )}
              style={along(mark.position)}
            >
              {vertical ? <span>{mark.label}</span> : null}
              <span
                className={cn(
                  mark.strong ? "bg-foreground" : "bg-(--wf-grid-line-strong)",
                  vertical ? "h-px w-1" : "h-1 w-px"
                )}
              />
              {vertical ? null : <span>{mark.label}</span>}
            </div>
          ))}
        </div>
      </div>
    ) : null

  const slider = (
    <div
      {...control.sliderProps}
      data-slot="fader-control"
      aria-label={ariaLabel}
      aria-labelledby={
        ariaLabelledBy ?? (label && !ariaLabel ? labelId : undefined)
      }
      className={cn(
        "group/fader relative shrink-0 touch-none rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-ring",
        vertical ? "w-7 cursor-ns-resize" : "h-7 cursor-ew-resize",
        "data-disabled:cursor-default"
      )}
    >
      <div
        className={cn(
          "absolute rounded-full bg-(--wf-control-track)",
          vertical
            ? "inset-y-0 left-1/2 w-1 -translate-x-1/2"
            : "inset-x-0 top-1/2 h-1 -translate-y-1/2"
        )}
      />
      <div
        className="absolute"
        style={vertical ? { inset: `${INSET}px 0` } : { inset: `0 ${INSET}px` }}
      >
        <div
          data-slot="fader-fill"
          className={cn(
            "absolute rounded-full bg-[var(--fader-fill,var(--wf-control-fill))]",
            vertical
              ? "left-1/2 w-1 -translate-x-1/2"
              : "top-1/2 h-1 -translate-y-1/2"
          )}
          style={
            vertical
              ? { bottom: -INSET, height: `calc(${percent} + ${INSET}px)` }
              : { left: -INSET, width: `calc(${percent} + ${INSET}px)` }
          }
        />
        <div
          data-slot="fader-cap"
          className={cn(
            "absolute rounded-[3px] border border-[color-mix(in_oklch,var(--foreground)_55%,var(--background))] bg-[var(--wf-fader-cap,color-mix(in_oklch,var(--foreground)_86%,var(--background)))] shadow-[0_1px_2px_rgb(0_0_0/0.35)] transition-[background-color] group-hover/fader:bg-[var(--wf-fader-cap-hover,var(--foreground))] group-data-dragging/fader:bg-[var(--wf-fader-cap-hover,var(--foreground))]",
            vertical
              ? "inset-x-0.5 translate-y-1/2"
              : "inset-y-0.5 -translate-x-1/2"
          )}
          style={
            vertical
              ? { bottom: percent, height: CAP }
              : { left: percent, width: CAP }
          }
        >
          <div
            className={cn(
              "absolute rounded-full bg-[var(--fader-fill,var(--wf-control-fill))]",
              vertical
                ? "inset-x-1 top-1/2 h-0.5 -translate-y-1/2"
                : "inset-y-1 left-1/2 w-0.5 -translate-x-1/2"
            )}
          />
        </div>
      </div>
    </div>
  )

  const meterSlot = meter ? (
    <div
      data-slot="fader-meter"
      className={cn("flex shrink-0", vertical ? "*:h-full" : "*:w-full")}
      style={
        vertical ? { padding: `${INSET}px 0` } : { padding: `0 ${INSET}px` }
      }
    >
      {meter}
    </div>
  ) : null

  return (
    <div
      data-slot="fader"
      data-orientation={orientation}
      data-disabled={disabled ? "" : undefined}
      className={cn(
        "relative inline-flex flex-col items-center gap-1.5 text-[10px] select-none data-disabled:opacity-50",
        vertical ? "h-44" : "w-48",
        className
      )}
      style={
        color
          ? ({ "--fader-fill": color, ...style } as React.CSSProperties)
          : style
      }
      {...props}
    >
      <div
        className={cn(
          "flex min-h-0 min-w-0 flex-1",
          vertical ? "flex-row gap-1" : "w-full flex-col gap-0.5"
        )}
      >
        {vertical ? scaleMarks : meterSlot}
        {slider}
        {vertical ? meterSlot : scaleMarks}
      </div>
      {showValue ? (
        <span
          data-slot="fader-value"
          className="leading-none whitespace-nowrap text-foreground tabular-nums"
          onDoubleClick={() => control.startEditing()}
        >
          {control.text}
        </span>
      ) : null}
      {label ? (
        <span
          id={labelId}
          data-slot="fader-label"
          className="max-w-full truncate leading-none text-muted-foreground"
        >
          {label}
        </span>
      ) : null}
      {control.editing ? (
        <ValueInput
          {...control.entryProps}
          aria-label={
            ariaLabel ?? (typeof label === "string" ? label : "Value")
          }
          className="absolute -bottom-[3px] left-1/2 z-10 h-4 w-14 -translate-x-1/2"
        />
      ) : null}
    </div>
  )
}

export { Fader }
export type { FaderProps }
