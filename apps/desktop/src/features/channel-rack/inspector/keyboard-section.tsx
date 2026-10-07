import { useEffect, useMemo, useRef } from "react"

import type { Channel } from "@/bindings"
import { noteName, PianoKeyboard } from "@/components/audio"
import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { useHint } from "@/lib/store/hint"
import { colorToCss, DEFAULT_KEY } from "@/lib/units"

import { auditionOff, auditionOn } from "../audition"
import { useRackStore, type KeyboardRange } from "../rack-store"
import { Section } from "./parts"

/**
 * Pixels per semitone: wide enough for a finger or a pointer on every key.
 * The keyboard is wider than the panel and scrolls sideways.
 */
const KEY_WIDTH = 522 / 48
/** A sampler is played around its root key; an instrument over six octaves. */
const AUTO = {
  sampler: { low: 36, high: 84 },
  instrument: { low: 24, high: 96 },
}
const WIDE = { low: 24, high: 96 }
const FULL = { low: 0, high: 127 }

/** The keys the keyboard shows for a kind of channel. */
function keyboardSpan(
  range: KeyboardRange,
  source: "sampler" | "instrument"
): { low: number; high: number } {
  if (range === "full") return FULL
  return range === "wide" ? WIDE : AUTO[source]
}

const span = (keys: { low: number; high: number }) =>
  `${noteName(keys.low)} to ${noteName(keys.high)}`

/** The right-click menu of the keyboard: its note names and how far it reaches. */
function keyboardMenu(source: "sampler" | "instrument"): ContextItem[] {
  const { keyboardLabels, keyboardRange, setKeyboardLabels, setKeyboardRange } =
    useRackStore.getState()
  const choice = (range: KeyboardRange, title: string): ContextItem => ({
    title: `${title} (${span(keyboardSpan(range, source))})`,
    checked: keyboardRange === range,
    run: () => setKeyboardRange(range),
  })
  return [
    { label: "Keyboard" },
    {
      title: "Note names",
      checked: keyboardLabels,
      run: () => setKeyboardLabels(!keyboardLabels),
    },
    contextSeparator,
    choice("auto", source === "sampler" ? "Around the root key" : "Six octaves"),
    // For an instrument the wide range is the one it has already.
    ...(source === "sampler" ? [choice("wide", "Six octaves")] : []),
    choice("full", "Every key"),
  ]
}

/** A keyboard that plays the channel at any pitch and loudness. */
export function KeyboardSection({ channel }: { channel: Channel }) {
  const { id, source } = channel
  // An instrument has no root key: the view starts around the key steps play.
  const rootKey = source.type === "sampler" ? source.rootKey : DEFAULT_KEY
  const playable = source.type === "instrument" || source.sample !== null
  const labels = useRackStore((state) => state.keyboardLabels)
  const range = useRackStore((state) => state.keyboardRange)
  const { low, high } = keyboardSpan(range, source.type)
  const width = Math.round((high - low) * KEY_WIDTH)
  const scroller = useRef<HTMLDivElement>(null)
  const active = useMemo(
    () => (source.type === "sampler" ? [rootKey] : []),
    [source.type, rootKey]
  )
  const hint = useHint(
    source.type === "instrument"
      ? `Play ${channel.name} at any pitch. Lower on a key is louder; drag across keys to slide. A step plays ${noteName(DEFAULT_KEY)}`
      : playable
        ? `Play ${channel.name} at any pitch. Lower on a key is louder; drag across keys to slide. The lit key is the root key, ${noteName(rootKey)}`
        : "Give the channel a sample to play it from the keyboard"
  )

  // Start with the root key in the middle of the view. The panel gets its
  // width a moment after it mounts, so this waits for the first real size.
  useEffect(() => {
    const element = scroller.current
    if (!element) return
    const observer = new ResizeObserver(() => {
      if (element.clientWidth === 0) return
      const share = (rootKey - low) / (high - low)
      element.scrollLeft = share * width - element.clientWidth / 2
      observer.disconnect()
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [id, rootKey, low, high, width])

  return (
    <Section title="Play">
      <ContextActions items={() => keyboardMenu(source.type)}>
        <div
          ref={scroller}
          data-slot="audition-keyboard"
          className="overflow-x-auto overflow-y-hidden rounded-[3px] pb-1"
          {...hint}
        >
          <PianoKeyboard
            lowKey={low}
            highKey={high}
            activeKeys={active}
            showLabels={labels}
            color={colorToCss(channel.color)}
            disabled={!playable}
            onNoteOn={(key, velocity) => auditionOn(id, key, velocity)}
            onNoteOff={(key) => auditionOff(id, key)}
            // The keyboard keeps the webview's menu away by itself. Here a
            // right-click goes on to the menu around it instead.
            onContextMenu={undefined}
            className="h-14"
            style={{ width }}
          />
        </div>
      </ContextActions>
    </Section>
  )
}
