import { useEffect, useMemo, useRef } from "react"

import type { TrackId } from "@/bindings"
import {
  Fader,
  formatGain,
  gainToFaderPosition,
  LevelMeter,
  type LevelMeterHandle,
} from "@/components/audio"
import { ValueContextItems } from "@/components/value-context-menu"
import { automationFeed, useAutomationMarker } from "@/features/automation/live"
import { meterFeed, useHint, useUiStore } from "@/lib/store"
import { cn } from "@/lib/utils"

import { trackValueItems } from "./menus"
import { CLIP_GAIN, resetPeak, subscribePeak } from "./peaks"
import { useTrackGroupGesture } from "./group-gesture"
import { StripWaveform } from "./waveform-meter"

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
 * light follows the held peak, so it is still lit when a strip that clipped
 * out of view scrolls back in.
 */
function StripMeter({ id, feed, orientation, wide }: StripMeterProps) {
  const meter = useRef<LevelMeterHandle>(null)
  const vertical = orientation === "vertical"

  useEffect(
    () =>
      subscribePeak(id, (peak) => meter.current?.setClipped(peak > CLIP_GAIN)),
    [id]
  )

  return (
    <LevelMeter
      ref={meter}
      subscribe={feed}
      orientation={orientation}
      taper={gainToFaderPosition}
      clipGain={CLIP_GAIN}
      clipLabel="Clip light. Click to clear"
      onClipChange={(clipped) => {
        if (!clipped) resetPeak(id)
      }}
      className={vertical ? cn("h-full", wide ? "w-4" : "w-3") : "h-1.5 w-full"}
    />
  )
}

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
  wide,
}: LevelSectionProps) {
  const meterMode = useUiStore((state) => state.mixerMeterMode)
  const level = useTrackGroupGesture(id, "volume")
  const feed = useMemo(() => meterFeed(index), [index])
  const hint = useHint(
    "Volume. Drag, or double-click for 0 dB. Hold Shift for fine steps"
  )
  // What the fader is bound to adds its own entries to the fader's menu,
  // and automation of it moves the fader while the song plays.
  const items = useMemo(() => trackValueItems(id, "volume"), [id])
  const live = useMemo(
    () => automationFeed({ type: "trackVolume", track: id }),
    [id]
  )
  const marker = useAutomationMarker({ type: "trackVolume", track: id })
  const upright = layout === "tall" || layout === "short"
  const meter = meterMode === "waveform" ? <StripWaveform id={id} vertical={upright} wide={wide} active={metering} /> : (
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
        <ValueContextItems items={items}>
          <Fader
            {...level}
            orientation="horizontal"
            // The lowest panel has no room for a full-height cap.
            size={layout === "bare" ? "sm" : "md"}
            ticks={false}
            aria-label={`${name} volume`}
            title={formatGain(level.value)}
            className="w-full"
            live={live}
            marker={marker}
            meter={meter}
            {...hint}
          />
        </ValueContextItems>
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
      <ValueContextItems items={items}>
        <Fader
          {...level}
          showValue
          aria-label={`${name} volume`}
          className="h-auto min-h-0 flex-1 [&_[data-slot=fader-value]]:font-readout [&_[data-slot=fader-value]]:text-[9px]"
          live={live}
          marker={marker}
          meter={meter}
          {...hint}
        />
      </ValueContextItems>
    </div>
  )
}
