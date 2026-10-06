import { useEffect, useEffectEvent } from "react"

import type { RealtimeFrame } from "@/bindings"
import { backend } from "@/lib/ipc"

/*
 * Playhead and meters arrive about 60 times a second. Putting them in React
 * state would render the app that often, so they live in a plain object and
 * components draw from it inside requestAnimationFrame.
 */

const latest: RealtimeFrame = {
  playing: false,
  tick: 0,
  meters: [],
  cpu: 0,
  xruns: 0,
  voices: 0,
}

type Listener = (frame: Readonly<RealtimeFrame>) => void

const listeners = new Set<Listener>()
let stopFeed: (() => void) | null = null
let animation: number | null = null
/** True when frames have arrived that no listener has drawn yet. */
let undrawn = false

function receive(frame: RealtimeFrame) {
  latest.playing = frame.playing
  latest.tick = frame.tick
  latest.cpu = frame.cpu
  latest.xruns = frame.xruns
  latest.voices = frame.voices
  // Meters are peaks since the previous frame. When the screen draws slower
  // than frames arrive, keep the highest so a short hit is not skipped.
  if (undrawn && latest.meters.length === frame.meters.length) {
    for (let index = 0; index < frame.meters.length; index += 1) {
      latest.meters[index] = Math.max(latest.meters[index], frame.meters[index])
    }
  } else {
    latest.meters = frame.meters.slice()
  }
  undrawn = true
}

function draw() {
  animation = requestAnimationFrame(draw)
  undrawn = false
  for (const listener of [...listeners]) listener(latest)
}

function start() {
  stopFeed ??= backend.subscribeRealtime(receive)
  animation ??= requestAnimationFrame(draw)
}

function stop() {
  stopFeed?.()
  stopFeed = null
  if (animation !== null) cancelAnimationFrame(animation)
  animation = null
}

/**
 * Calls `listener` once per animation frame with the newest values. The
 * frame object is reused, so read from it and do not keep it.
 */
export function subscribeRealtime(listener: Listener): () => void {
  listeners.add(listener)
  start()
  return () => {
    listeners.delete(listener)
    if (listeners.size === 0) stop()
  }
}

/** The newest frame, for code that draws on its own schedule. */
export function realtimeFrame(): Readonly<RealtimeFrame> {
  return latest
}

/**
 * Runs `draw` once per animation frame with the newest realtime values. Draw
 * to a canvas or set a style in it; do not set React state.
 */
export function useRealtime(draw: Listener) {
  const onFrame = useEffectEvent(draw)
  useEffect(() => subscribeRealtime((frame) => onFrame(frame)), [])
}

/**
 * Peak levels of one mixer track as linear gain, left and right.
 * `trackIndex` is the track's position in the mixer; the master is 0.
 */
export function useMeter(
  trackIndex: number,
  draw: (left: number, right: number) => void
) {
  useRealtime((frame) => {
    const left = frame.meters[trackIndex * 2] ?? 0
    const right = frame.meters[trackIndex * 2 + 1] ?? 0
    draw(left, right)
  })
}

/**
 * A feed of one mixer track's peak levels, in the shape the level meter in
 * `components/audio` takes as its `subscribe` prop:
 *
 *     <LevelMeter subscribe={meterFeed(trackIndex)} />
 */
export function meterFeed(trackIndex: number) {
  return (listener: (left: number, right: number) => void) =>
    subscribeRealtime((frame) =>
      listener(
        frame.meters[trackIndex * 2] ?? 0,
        frame.meters[trackIndex * 2 + 1] ?? 0
      )
    )
}

/**
 * The playhead in ticks. In pattern mode it is the position inside the
 * pattern, in song mode the position on the playlist.
 */
export function usePlayhead(draw: (tick: number, playing: boolean) => void) {
  useRealtime((frame) => draw(frame.tick, frame.playing))
}
