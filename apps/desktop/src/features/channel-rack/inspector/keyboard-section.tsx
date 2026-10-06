import { useEffect, useMemo, useRef } from "react"

import type { Channel } from "@/bindings"
import { noteName, PianoKeyboard } from "@/components/audio"
import { useHint } from "@/lib/store/hint"
import { colorToCss } from "@/lib/units"

import { auditionOff, auditionOn } from "../audition"
import { Section } from "./parts"

const LOW_KEY = 36
const HIGH_KEY = 84
/** Wide enough for a finger or a pointer on every key; it scrolls sideways. */
const KEYBOARD_WIDTH = 522

/** A keyboard that plays the channel at any pitch and loudness. */
export function KeyboardSection({ channel }: { channel: Channel }) {
  const { id } = channel
  const rootKey = channel.source.rootKey
  const hasSample = channel.source.sample !== null
  const scroller = useRef<HTMLDivElement>(null)
  const active = useMemo(() => [rootKey], [rootKey])
  const hint = useHint(
    hasSample
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
      const share = (rootKey - LOW_KEY) / (HIGH_KEY - LOW_KEY)
      element.scrollLeft = share * KEYBOARD_WIDTH - element.clientWidth / 2
      observer.disconnect()
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [id, rootKey])

  return (
    <Section title="Play">
      <div
        ref={scroller}
        className="overflow-x-auto overflow-y-hidden rounded-[3px] pb-1"
        {...hint}
      >
        <PianoKeyboard
          lowKey={LOW_KEY}
          highKey={HIGH_KEY}
          activeKeys={active}
          color={colorToCss(channel.color)}
          disabled={!hasSample}
          onNoteOn={(key, velocity) => auditionOn(id, key, velocity)}
          onNoteOff={(key) => auditionOff(id, key)}
          className="h-14"
          style={{ width: KEYBOARD_WIDTH }}
        />
      </div>
    </Section>
  )
}
