// SPDX-License-Identifier: MIT
import * as React from "react"

import { cn } from "@/lib/utils"

import {
  observeCanvas,
  resolveColors,
  subscribeTheme,
  type CanvasSize,
} from "./canvas"
import { dbToGain, gainToDb } from "./units"

/** Levels rest here when nothing is playing. */
export const METER_FLOOR_DB = -100

export type MeterChannel = {
  /** The displayed level in dB. */
  level: number
  /** The held peak in dB. */
  peak: number
  /** Seconds since the peak was last pushed up. */
  peakAge: number
  clipped: boolean
}

export type MeterBallistics = {
  /** How fast the level falls, in dB per second. */
  releaseDbPerSecond: number
  /** How long a peak stays before it starts to fall, in seconds. */
  peakHoldSeconds: number
}

export function createMeterChannel(): MeterChannel {
  return {
    level: METER_FLOOR_DB,
    peak: METER_FLOOR_DB,
    peakAge: 0,
    clipped: false,
  }
}

/**
 * Moves one channel forward by `seconds`. The level jumps up to the input at
 * once and falls at the release rate; the peak holds, then falls the same way.
 */
export function advanceMeter(
  channel: MeterChannel,
  inputDb: number,
  seconds: number,
  { releaseDbPerSecond, peakHoldSeconds }: MeterBallistics
): void {
  const input = Number.isNaN(inputDb) ? METER_FLOOR_DB : inputDb
  channel.level = Math.max(
    METER_FLOOR_DB,
    input,
    channel.level - releaseDbPerSecond * seconds
  )
  if (input >= channel.peak) {
    channel.peak = input
    channel.peakAge = 0
  } else {
    channel.peakAge += seconds
    const falling = Math.min(seconds, channel.peakAge - peakHoldSeconds)
    if (falling > 0) {
      channel.peak -= releaseDbPerSecond * falling
    }
  }
  channel.peak = Math.max(channel.peak, channel.level)
}

export type LevelMeterHandle = {
  /** Feeds linear peak values. Call it as often as values arrive. */
  set: (left: number, right?: number) => void
  clearClip: () => void
  /** Drops the levels, the held peaks and the clip indicator. */
  reset: () => void
}

type MeterConfig = {
  channels: 1 | 2
  vertical: boolean
  minDb: number
  maxDb: number
  midDb: number
  highDb: number
  taper?: (gain: number) => number
  releaseDbPerSecond: number
  peakHoldMs: number
  clipGain: number
  showClip: boolean
  onClipChange?: (clipped: boolean) => void
}

const COLORS = {
  bg: "var(--wf-meter-bg)",
  low: "var(--wf-meter-low)",
  mid: "var(--wf-meter-mid)",
  high: "var(--wf-meter-high)",
}

// The length of the clip light and of the space under it, in CSS pixels.
const CLIP_LENGTH = 4
const CLIP_GAP = 1

// In a fader's meter slot the clip light sits past the end of the travel, so
// the bars alone span it and line up with the fader's marks. The 5px is
// CLIP_LENGTH + CLIP_GAP.
const IN_FADER = {
  vertical:
    "[[data-slot=fader-meter]>&]:-mt-[5px] [[data-slot=fader-meter]>&]:h-[calc(100%+5px)]",
  horizontal:
    "[[data-slot=fader-meter]>&]:-mr-[5px] [[data-slot=fader-meter]>&]:w-[calc(100%+5px)]",
}

function createMeter(
  root: HTMLElement,
  canvas: HTMLCanvasElement,
  button: HTMLButtonElement,
  getConfig: () => MeterConfig
) {
  const context = canvas.getContext("2d")
  const channels = [createMeterChannel(), createMeterChannel()]
  const pending = [0, 0]
  let size: CanvasSize | null = null
  let colors: typeof COLORS | null = null
  let frame = 0
  let lastTime = 0
  let visible = true
  let stale = true
  let drawn = ""

  const isClipped = () => channels[0].clipped || channels[1].clipped

  function showClipState() {
    const clipped = isClipped()
    root.toggleAttribute("data-clipped", clipped)
    // Only a lit clip light is worth a tab stop.
    button.tabIndex = clipped ? 0 : -1
    button.setAttribute("aria-hidden", clipped ? "false" : "true")
    getConfig().onClipChange?.(clipped)
  }

  function schedule() {
    // Out of view the loop rests, except for the one frame that repaints
    // after a resize or a theme change.
    if (frame === 0 && context && (visible || stale)) {
      frame = requestAnimationFrame(tick)
    }
  }

  function invalidate() {
    stale = true
    schedule()
  }

  function position(config: MeterConfig, db: number): number {
    const place = config.taper
      ? config.taper(dbToGain(db))
      : (db - config.minDb) / (config.maxDb - config.minDb)
    return Math.min(1, Math.max(0, place))
  }

  function draw(config: MeterConfig) {
    if (!context || !size) {
      return
    }
    const { pixelWidth, pixelHeight, dpr } = size
    const count = config.channels
    const line = Math.max(1, Math.round(dpr))
    const clip = config.showClip ? Math.round(CLIP_LENGTH * dpr) : 0
    const clipGap = config.showClip
      ? Math.max(1, Math.round(CLIP_GAP * dpr))
      : 0
    const full = config.vertical ? pixelHeight : pixelWidth
    const across = config.vertical ? pixelWidth : pixelHeight
    const length = Math.max(1, full - clip - clipGap)
    const gap = count === 2 ? line : 0
    const bar = Math.max(1, Math.floor((across - gap) / count))
    const midAt = Math.round(position(config, config.midDb) * length)
    const highAt = Math.round(position(config, config.highDb) * length)

    const levels = channels.map((channel) =>
      Math.round(position(config, channel.level) * length)
    )
    const peaks = channels.map((channel) =>
      config.peakHoldMs > 0
        ? Math.round(position(config, channel.peak) * length)
        : 0
    )
    const signature = [
      pixelWidth,
      pixelHeight,
      count,
      config.vertical,
      midAt,
      highAt,
      levels,
      peaks,
      channels.map((channel) => channel.clipped),
    ].join()
    if (!stale && signature === drawn) {
      return
    }
    stale = false
    drawn = signature
    colors ??= resolveColors(root, COLORS)
    const palette = colors

    // Paints a stretch of one bar, measured from its quiet end.
    const paint = (index: number, from: number, to: number, color: string) => {
      if (to <= from) {
        return
      }
      const offset = index * (bar + gap)
      context.fillStyle = color
      if (config.vertical) {
        context.fillRect(offset, pixelHeight - to, bar, to - from)
      } else {
        context.fillRect(from, offset, to - from, bar)
      }
    }
    const zone = (at: number) =>
      at > highAt ? palette.high : at > midAt ? palette.mid : palette.low

    context.clearRect(0, 0, pixelWidth, pixelHeight)
    for (let index = 0; index < count; index += 1) {
      const level = levels[index]
      paint(index, 0, length, palette.bg)
      paint(index, 0, Math.min(level, midAt), palette.low)
      paint(index, midAt, Math.min(level, highAt), palette.mid)
      paint(index, highAt, level, palette.high)
      const peak = peaks[index]
      if (peak > level && peak >= line) {
        paint(index, peak - line, peak, zone(peak))
      }
      if (clip > 0) {
        paint(
          index,
          length + clipGap,
          full,
          channels[index].clipped ? palette.high : palette.bg
        )
      }
    }
  }

  function tick(time: number) {
    frame = 0
    const config = getConfig()
    const seconds = lastTime === 0 ? 0 : Math.max(0, (time - lastTime) / 1000)
    lastTime = time
    let moving = false
    for (let index = 0; index < 2; index += 1) {
      const channel = channels[index]
      advanceMeter(channel, gainToDb(pending[index]), seconds, {
        releaseDbPerSecond: config.releaseDbPerSecond,
        peakHoldSeconds: config.peakHoldMs / 1000,
      })
      pending[index] = 0
      if (channel.level > METER_FLOOR_DB || channel.peak > METER_FLOOR_DB) {
        moving = true
      }
    }
    draw(config)
    if (moving && visible) {
      schedule()
    } else {
      // At rest the loop stops; the next set() starts it again.
      lastTime = 0
    }
  }

  function set(left: number, right: number = left) {
    const config = getConfig()
    const values = [left, config.channels === 2 ? right : left]
    let clippedNow = false
    let audible = false
    for (let index = 0; index < 2; index += 1) {
      const gain = Math.abs(values[index])
      if (Number.isNaN(gain)) {
        continue
      }
      pending[index] = Math.max(pending[index], gain)
      audible ||= gainToDb(gain) > METER_FLOOR_DB
      if (gain > config.clipGain && !channels[index].clipped) {
        channels[index].clipped = true
        clippedNow = true
      }
    }
    if (clippedNow) {
      showClipState()
      stale = true
    }
    if (audible || clippedNow) {
      schedule()
    }
  }

  function clearClip() {
    if (!isClipped()) {
      return
    }
    channels[0].clipped = false
    channels[1].clipped = false
    showClipState()
    invalidate()
  }

  function reset() {
    const wasClipped = isClipped()
    channels[0] = createMeterChannel()
    channels[1] = createMeterChannel()
    pending[0] = 0
    pending[1] = 0
    if (wasClipped) {
      showClipState()
    }
    invalidate()
  }

  const stopCanvas = observeCanvas(canvas, (next) => {
    size = next
    invalidate()
  })
  const stopTheme = subscribeTheme(() => {
    colors = null
    invalidate()
  })
  // A meter scrolled out of view keeps its state but stops drawing.
  const viewObserver =
    typeof IntersectionObserver === "undefined"
      ? null
      : new IntersectionObserver(
          (entries) => {
            visible = entries[entries.length - 1].isIntersecting
            if (visible) {
              invalidate()
            }
          },
          // Start a little early so a meter is live as it scrolls in.
          { rootMargin: "64px" }
        )
  viewObserver?.observe(root)

  return {
    set,
    clearClip,
    reset,
    invalidate,
    destroy() {
      stopCanvas()
      stopTheme()
      viewObserver?.disconnect()
      if (frame !== 0) {
        cancelAnimationFrame(frame)
        frame = 0
      }
    },
  }
}

type LevelMeterProps = Omit<React.ComponentProps<"div">, "ref" | "children"> & {
  ref?: React.Ref<LevelMeterHandle>
  /** One bar or a left and right pair. */
  channels?: 1 | 2
  orientation?: "vertical" | "horizontal"
  /**
   * Another way to feed the meter: called on mount with a listener to call
   * with new peak values; returns a function that stops the feed.
   */
  subscribe?: (listener: (left: number, right?: number) => void) => () => void
  /** The bottom and top of the scale in dB. */
  minDb?: number
  maxDb?: number
  /**
   * Replaces the dB scale: maps linear gain to a 0 to 1 position. Pass
   * `gainToFaderPosition` to line the meter up with a fader's marks.
   */
  taper?: (gain: number) => number
  /** Where the yellow and the red zones begin, in dB. */
  midDb?: number
  highDb?: number
  releaseDbPerSecond?: number
  /** How long the peak line holds. 0 hides it. */
  peakHoldMs?: number
  /** Linear gain above which the clip light latches. */
  clipGain?: number
  showClip?: boolean
  onClipChange?: (clipped: boolean) => void
}

/**
 * A peak meter. It is fed outside React, through its ref or `subscribe`, and
 * does its own ballistics in an animation frame loop that stops at rest.
 */
function LevelMeter({
  ref,
  className,
  channels = 2,
  orientation = "vertical",
  subscribe,
  minDb = -60,
  maxDb = 6,
  taper,
  midDb = -12,
  highDb = 0,
  releaseDbPerSecond = 20,
  peakHoldMs = 1000,
  clipGain = 1,
  showClip = true,
  onClipChange,
  ...props
}: LevelMeterProps) {
  const vertical = orientation === "vertical"
  const config: MeterConfig = {
    channels,
    vertical,
    minDb,
    maxDb,
    midDb,
    highDb,
    taper,
    releaseDbPerSecond,
    peakHoldMs,
    clipGain,
    showClip,
    onClipChange,
  }

  const rootRef = React.useRef<HTMLDivElement>(null)
  const canvasRef = React.useRef<HTMLCanvasElement>(null)
  const buttonRef = React.useRef<HTMLButtonElement>(null)
  const meter = React.useRef<ReturnType<typeof createMeter> | null>(null)
  const latest = React.useRef(config)

  React.useLayoutEffect(() => {
    latest.current = config
    meter.current?.invalidate()
  })

  React.useEffect(() => {
    const root = rootRef.current
    const canvas = canvasRef.current
    const button = buttonRef.current
    if (!root || !canvas || !button) {
      return undefined
    }
    const created = createMeter(root, canvas, button, () => latest.current)
    meter.current = created
    return () => {
      created.destroy()
      meter.current = null
    }
  }, [])

  React.useImperativeHandle(
    ref,
    () => ({
      set: (left, right) => meter.current?.set(left, right),
      clearClip: () => meter.current?.clearClip(),
      reset: () => meter.current?.reset(),
    }),
    []
  )

  React.useEffect(() => {
    return subscribe?.((left, right) => meter.current?.set(left, right))
  }, [subscribe])

  return (
    <div
      ref={rootRef}
      data-slot="level-meter"
      data-orientation={orientation}
      className={cn(
        "relative shrink-0 overflow-hidden rounded-[2px]",
        vertical ? "h-32" : "w-32",
        vertical
          ? channels === 2
            ? "w-2.5"
            : "w-1.5"
          : channels === 2
            ? "h-2.5"
            : "h-1.5",
        showClip && (vertical ? IN_FADER.vertical : IN_FADER.horizontal),
        className
      )}
      {...props}
    >
      <canvas
        ref={canvasRef}
        aria-hidden="true"
        className="absolute inset-0 block size-full"
      />
      <button
        ref={buttonRef}
        type="button"
        tabIndex={-1}
        aria-hidden="true"
        aria-label="Clear clip indicator"
        className="absolute inset-0 size-full cursor-default rounded-[2px] outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
        onClick={() => meter.current?.clearClip()}
      />
    </div>
  )
}

export { LevelMeter }
export type { LevelMeterProps }
