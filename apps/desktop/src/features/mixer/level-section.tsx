import { useEffect, useMemo, useRef } from "react"

import type { TrackId } from "@/bindings"
import {
  Fader,
  FADER_DB_TICKS,
  formatGain,
  gainToFaderPosition,
  LevelMeter,
  type FaderTick,
} from "@/components/audio"
import { meterFeed, useHint } from "@/lib/store"
import { cn } from "@/lib/utils"

import { clampGain } from "./operations"
import { CLIP_GAIN, resetPeak, subscribePeak } from "./peaks"
import { useGestureValue } from "./use-gesture-value"

/** A held peak as the readout prints it: "−3.2", "+1.5" or "−∞". */
export function formatPeak(peak: number): string {
  return formatGain(peak).replace(" dB", "")
}

/**
 * The loudest level the track has reached, in dB. It turns red once the
 * track has gone over 0 dB and stays that way until it is clicked.
 */
function PeakReadout({ id, className }: { id: TrackId; className?: string }) {
  const button = useRef<HTMLButtonElement>(null)
  const text = useRef<HTMLSpanElement>(null)
  const hint = useHint("Highest level since the last reset. Click to reset")

  useEffect(
    () =>
      subscribePeak(id, (peak) => {
        if (text.current) text.current.textContent = formatPeak(peak)
        button.current?.toggleAttribute("data-clipped", peak > CLIP_GAIN)
      }),
    [id]
  )

  return (
    <button
      ref={button}
      type="button"
      data-slot="track-peak"
      className={cn(
        "h-3.5 shrink-0 rounded-[3px] bg-display font-readout text-[9px] leading-none text-display-foreground shadow-[inset_0_1px_1px_rgb(0_0_0/0.4)] outline-none hover:brightness-125 focus-visible:ring-2 focus-visible:ring-ring data-clipped:bg-(--wf-meter-high) data-clipped:text-[oklch(0.16_0_0)]",
        className
      )}
      onClick={() => resetPeak(id)}
      {...hint}
    >
      <span className="sr-only">Peak </span>
      <span ref={text} />
    </button>
  )
}

type StripMeterProps = {
  id: TrackId
  /** Where the track's levels come from. Leave out to rest the meter. */
  feed?: ReturnType<typeof meterFeed>
  orientation: "vertical" | "horizontal"
  wide: boolean
}

/**
 * The stereo meter beside the fader, on the fader's own dB scale. Its clip
 * light sits just past the top of the scale and follows the held peak, so
 * it is still lit when a strip that clipped out of view scrolls back in.
 */
function StripMeter({ id, feed, orientation, wide }: StripMeterProps) {
  const light = useRef<HTMLButtonElement>(null)
  const vertical = orientation === "vertical"

  useEffect(
    () =>
      subscribePeak(id, (peak) => {
        const clipped = peak > CLIP_GAIN
        light.current?.toggleAttribute("data-clipped", clipped)
        // Only a lit clip light is worth a tab stop.
        if (light.current) light.current.tabIndex = clipped ? 0 : -1
      }),
    [id]
  )

  return (
    <div className={cn("relative", vertical ? "h-full" : "w-full")}>
      <LevelMeter
        subscribe={feed}
        orientation={orientation}
        taper={gainToFaderPosition}
        showClip={false}
        className={
          vertical ? cn("h-full", wide ? "w-4" : "w-3") : "h-1.5 w-full"
        }
      />
      <button
        ref={light}
        type="button"
        tabIndex={-1}
        data-slot="track-clip"
        aria-label="Clip light. Click to clear"
        className={cn(
          "absolute rounded-[1px] bg-(--wf-meter-bg) outline-none after:absolute after:-inset-1 focus-visible:ring-2 focus-visible:ring-ring data-clipped:bg-(--wf-meter-high)",
          vertical ? "inset-x-0 -top-[5px] h-1" : "inset-y-0 -right-[5px] w-1"
        )}
        onClick={() => resetPeak(id)}
      />
    </div>
  )
}

// With little travel the labels of neighboring dB marks run into each
// other, so a short fader labels fewer of them. The lines all stay.
const SPARSE_TICKS: readonly FaderTick[] = FADER_DB_TICKS.map((tick) =>
  tick.label === "−6" || tick.label === "−∞"
    ? { value: tick.value, strong: tick.strong }
    : tick
)

export type LevelLayout =
  /** An upright fader with the peak readout above it. */
  | "tall"
  /** An upright fader alone. */
  | "short"
  /** A fader across the strip, with the readouts in a line under it. */
  | "flat"
  /** A fader across the strip alone. */
  | "bare"

type LevelSectionProps = {
  id: TrackId
  name: string
  volume: number
  /** The track's place in the mixer, which is its place in the meters. */
  index: number
  /** False while the strip is kept mounted out of view. */
  metering: boolean
  layout: LevelLayout
  /** Label only some of the dB marks, for a fader with little travel. */
  sparseScale: boolean
  wide: boolean
}

/** The fader with its meter, and the peak and fader readouts. */
export function LevelSection({
  id,
  name,
  volume,
  index,
  metering,
  layout,
  sparseScale,
  wide,
}: LevelSectionProps) {
  const level = useGestureValue(
    volume,
    (value) => ({ type: "updateMixerTrack", id, patch: { volume: value } }),
    clampGain
  )
  const feed = useMemo(() => meterFeed(index), [index])
  const hint = useHint(
    "Volume. Drag, or double-click for 0 dB. Hold Shift for fine steps"
  )
  const upright = layout === "tall" || layout === "short"
  const meter = (
    <StripMeter
      id={id}
      feed={metering ? feed : undefined}
      orientation={upright ? "vertical" : "horizontal"}
      wide={wide}
    />
  )

  if (!upright) {
    return (
      <div className="flex shrink-0 flex-col gap-1 px-1">
        <Fader
          {...level}
          orientation="horizontal"
          ticks={false}
          aria-label={`${name} volume`}
          title={formatGain(level.value)}
          className={cn(
            "w-full",
            // The lowest panel has no room for a full-height cap.
            layout === "bare" && "[&_[data-slot=fader-control]]:h-5"
          )}
          meter={meter}
          {...hint}
        />
        {layout === "flat" && (
          <div className="flex h-3.5 items-center justify-between gap-1">
            <PeakReadout id={id} className="w-9" />
            <span
              data-slot="fader-value"
              className="font-readout text-[9px] leading-none whitespace-nowrap"
            >
              {formatGain(level.value)}
            </span>
          </div>
        )}
      </div>
    )
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col items-center gap-1.5">
      {layout === "tall" && <PeakReadout id={id} className="w-12" />}
      <Fader
        {...level}
        showValue
        ticks={sparseScale ? SPARSE_TICKS : undefined}
        aria-label={`${name} volume`}
        className="h-auto min-h-0 flex-1 [&_[data-slot=fader-value]]:font-readout [&_[data-slot=fader-value]]:text-[9px]"
        meter={meter}
        {...hint}
      />
    </div>
  )
}
