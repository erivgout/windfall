import type { Channel, ChannelId, SamplerPatch } from "@/bindings"
import {
  faderTaper,
  formatGain,
  gainUnit,
  Knob,
  noteName,
  NumberField,
  parseNumber,
  semitonesUnit,
} from "@/components/audio"
import { useHint } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { clamp, DEFAULT_KEY, MAX_GAIN } from "@/lib/units"

import { findChannel } from "../channel-ops"
import { joinTune, splitTune } from "../steps"
import { useGestureValue } from "../use-gesture-value"
import { useLiveHint } from "../use-live-hint"
import { Section, SwitchRow } from "./parts"

const MAX_TUNE = 48
const MAX_KEY = 127
const MAX_CUT_GROUP = 255

const NOTE_OFFSETS: Record<string, number> = {
  c: 0,
  d: 2,
  e: 4,
  f: 5,
  g: 7,
  a: 9,
  b: 11,
}

/** Reads "C5", "f#3", "Bb4" or a key number. FL's naming: key 60 is C5. */
export function parseNoteName(text: string): number | null {
  const match = /^\s*([a-g])\s*([#b]?)\s*(-?\d+)\s*$/i.exec(text)
  if (!match) return parseNumber(text)
  const accidental = match[2] === "#" ? 1 : match[2] === "" ? 0 : -1
  return (
    Number(match[3]) * 12 + NOTE_OFFSETS[match[1].toLowerCase()] + accidental
  )
}

const formatKey = (key: number) => noteName(Math.round(key))
const formatCents = (cents: number) => {
  const rounded = Math.round(cents)
  return `${rounded > 0 ? "+" : rounded < 0 ? "−" : ""}${Math.abs(rounded)} ct`
}

function storedTune(id: ChannelId) {
  return splitTune(findChannel(id)?.source.tune ?? 0)
}

/** Pitch, level and how the channel's notes cut each other off. */
export function SoundSection({ channel }: { channel: Channel }) {
  const { id, source } = channel
  const send = (patch: SamplerPatch) =>
    dispatch({ type: "updateSampler", id, patch })
  const { semitones, cents } = splitTune(source.tune)

  const root = useGestureValue(source.rootKey, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: { rootKey: clamp(Math.round(value), 0, MAX_KEY) },
    })
  )
  // Tune and Fine are two views of one number, so each reads the other half
  // from the project at the moment it sends.
  const tune = useGestureValue(semitones, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: { tune: joinTune(value, storedTune(id).cents, MAX_TUNE) },
    })
  )
  const fine = useGestureValue(cents, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: { tune: joinTune(storedTune(id).semitones, value, MAX_TUNE) },
    })
  )
  const gain = useGestureValue(source.gain, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: { gain: clamp(value, 0, MAX_GAIN) },
    })
  )
  const cutGroup = useGestureValue(source.cutGroup, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: { cutGroup: clamp(Math.round(value), 0, MAX_CUT_GROUP) },
    })
  )

  const rootHint = useLiveHint(
    `Root key: ${formatKey(root.value)}. The key that plays the sample at its own pitch. Double-click for C5`
  )
  const tuneHint = useLiveHint(
    `Tune: ${semitonesUnit.format(tune.value)}. Whole semitones up or down. Double-click to reset`
  )
  const fineHint = useLiveHint(
    `Fine tune: ${formatCents(fine.value)}. Hundredths of a semitone. Double-click to reset`
  )
  const gainHint = useLiveHint(
    `Sample gain: ${formatGain(gain.value)}. Level of the sample before the channel volume. Double-click for 0 dB`
  )
  const cutHint = useHint(
    "Cut group: channels with the same number stop each other, like an open and a closed hat. 0 means none"
  )

  return (
    <Section title="Sound">
      <div className="grid grid-cols-4 justify-items-center gap-1">
        <Knob
          label="Root"
          showValue
          min={0}
          max={MAX_KEY}
          step={1}
          defaultValue={DEFAULT_KEY}
          format={formatKey}
          parse={parseNoteName}
          {...root}
          {...rootHint}
        />
        <Knob
          label="Tune"
          showValue
          bipolar
          min={-MAX_TUNE}
          max={MAX_TUNE}
          step={1}
          defaultValue={0}
          {...semitonesUnit}
          {...tune}
          {...tuneHint}
        />
        <Knob
          label="Fine"
          showValue
          bipolar
          min={-50}
          max={50}
          step={1}
          defaultValue={0}
          format={formatCents}
          {...fine}
          {...fineHint}
        />
        <Knob
          label="Gain"
          showValue
          min={0}
          max={MAX_GAIN}
          scale={faderTaper}
          defaultValue={1}
          {...gainUnit}
          {...gain}
          {...gainHint}
        />
      </div>
      <div className="mt-1 flex flex-col gap-1.5">
        <SwitchRow
          label="Reverse"
          hint="Reverse: play the sample backwards"
          checked={source.reverse}
          onCheckedChange={(reverse) => void send({ reverse })}
        />
        <SwitchRow
          label="Cut itself"
          hint="Cut itself: a new note stops the one still ringing, so hits never overlap"
          checked={source.cutSelf}
          onCheckedChange={(cutSelf) => void send({ cutSelf })}
        />
        <div
          className="flex h-6 items-center justify-between gap-2"
          {...cutHint}
        >
          <span className="truncate text-foreground/85">Cut group</span>
          <span className="flex items-center gap-1.5">
            {cutGroup.value === 0 && (
              <span className="text-muted-foreground">none</span>
            )}
            <NumberField
              size="sm"
              aria-label="Cut group"
              min={0}
              max={MAX_CUT_GROUP}
              step={1}
              defaultValue={0}
              className="w-12 justify-end font-readout"
              {...cutGroup}
            />
          </span>
        </div>
      </div>
    </Section>
  )
}
