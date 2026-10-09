import { useState } from "react"

import type { ChannelId, SamplerPatch } from "@/bindings"
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
import { Button } from "@/components/ui/button"
import type { SamplerChannel } from "@/lib/channel-source"
import { useHint } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { clamp, DEFAULT_KEY, MAX_GAIN } from "@/lib/units"

import { findChannel } from "../channel-ops"
import { joinTune, splitTune } from "../steps"
import { useGestureValue } from "../use-gesture-value"
import { nextCutGroupPreset } from "./cut-group-preset-step"
import { CUT_GROUP_PRESETS, nextCutGroup } from "./cut-group-presets"
import { nextSamplerFineScale } from "./fine-cents-scale"
import { nextSamplerFinePreset } from "./fine-tune-preset-step"
import { FINE_TUNE_PRESETS, nextSamplerFine } from "./fine-tune-presets"
import { Section, SwitchRow } from "./parts"
import { nextRootKeyPreset } from "./root-key-preset-step"
import { nextRootKey, ROOT_KEY_PRESETS } from "./root-key-presets"
import { nextRootKeyScale } from "./root-key-scale"
import { nextSampleGainPreset } from "./sample-gain-preset-step"
import { nextSampleGain, SAMPLE_GAIN_PRESETS } from "./sample-gain-presets"
import { nextSampleGainScale } from "./sample-gain-scale"
import { nextSamplerTunePreset } from "./tune-preset-step"
import { nextSamplerTune, TUNE_PRESETS } from "./tune-presets"
import { nextSamplerTuneScale } from "./tune-scale"

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

/** The channel's tuning as the project has it right now. */
function storedTune(id: ChannelId): number {
  const source = findChannel(id)?.source
  return source?.type === "sampler" ? source.tune : 0
}

/** Pitch, level and how the channel's notes cut each other off. */
export function SoundSection({ channel }: { channel: SamplerChannel }) {
  const { id, source } = channel
  const send = (patch: SamplerPatch) =>
    dispatch({ type: "updateSampler", id, patch })
  // Tune and Fine are two views of one number, and a tuning half way
  // between two semitones can be read as either. The semitone on show is
  // kept here, so Fine at +50 or -50 does not flip Tune to the neighbour.
  const [coarse, setCoarse] = useState(() => splitTune(source.tune).semitones)
  const { semitones, cents } = splitTune(source.tune, coarse)
  if (semitones !== coarse) setCoarse(semitones)

  const root = useGestureValue(source.rootKey, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: { rootKey: clamp(Math.round(value), 0, MAX_KEY) },
    })
  )
  // Each knob reads the other half from the project at the moment it
  // sends, split around the semitone on show.
  const tune = useGestureValue(semitones, (value, sendInGesture) => {
    const { cents: kept } = splitTune(storedTune(id), coarse)
    // The knob now shows this semitone, so it is the one to keep.
    setCoarse(value)
    return sendInGesture({
      type: "updateSampler",
      id,
      patch: { tune: joinTune(value, kept, MAX_TUNE) },
    })
  })
  const fine = useGestureValue(cents, (value, sendInGesture) =>
    sendInGesture({
      type: "updateSampler",
      id,
      patch: {
        tune: joinTune(
          splitTune(storedTune(id), coarse).semitones,
          value,
          MAX_TUNE
        ),
      },
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

  const rootHint = useHint(
    `Root key: ${formatKey(root.value)}. The key that plays the sample at its own pitch. Double-click for C5`
  )
  const tuneHint = useHint(
    `Tune: ${semitonesUnit.format(tune.value)}. Whole semitones up or down. Double-click to reset`
  )
  const fineHint = useHint(
    `Fine tune: ${formatCents(fine.value)}. Hundredths of a semitone. Double-click to reset`
  )
  const gainHint = useHint(
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
      <div className="mt-1 flex flex-wrap gap-1">
        {ROOT_KEY_PRESETS.map(({ label, rootKey: preset }) => (
          <Button
            key={label}
            variant="outline"
            size="sm"
            disabled={nextRootKey(source.rootKey, preset) === null}
            onClick={() => {
              const latest = findChannel(id)?.source
              if (latest?.type !== "sampler") return
              if (nextRootKey(latest.rootKey, preset) !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { rootKey: preset },
                })
              }
            }}
          >
            {label}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["previous", "next"] as const).map((direction) => (
          <Button
            key={direction}
            variant="outline"
            size="sm"
            aria-label={
              direction === "previous"
                ? "Choose the previous root key preset"
                : "Choose the next root key preset"
            }
            disabled={nextRootKeyPreset(source.rootKey, direction) === null}
            onClick={() => {
              const latest = findChannel(id)?.source
              if (latest?.type !== "sampler") return
              const next = nextRootKeyPreset(latest.rootKey, direction)
              if (next === null) return
              void dispatch({
                type: "updateSampler",
                id,
                patch: { rootKey: next },
              })
            }}
          >
            {direction === "previous" ? "Previous" : "Next"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            aria-label={
              factor === "half"
                ? "Halve root distance from C5"
                : "Double root distance from C5"
            }
            disabled={nextRootKeyScale(source.rootKey, factor) === null}
            onClick={() => {
              const latest = findChannel(id)?.source
              if (latest?.type !== "sampler") return
              const next = nextRootKeyScale(latest.rootKey, factor)
              if (next !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { rootKey: next },
                })
              }
            }}
          >
            {factor === "half" ? "Half" : "Double"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {TUNE_PRESETS.map(({ label, semitones: preset }) => (
          <Button
            key={label}
            variant="outline"
            size="sm"
            disabled={nextSamplerTune(source.tune, cents, preset) === null}
            onClick={() => {
              const next = nextSamplerTune(source.tune, cents, preset)
              if (next !== null) {
                setCoarse(preset)
                void send({ tune: next })
              }
            }}
          >
            {label}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["previous", "next"] as const).map((direction) => (
          <Button
            key={direction}
            variant="outline"
            size="sm"
            aria-label={
              direction === "previous"
                ? "Choose the previous sampler tune"
                : "Choose the next sampler tune"
            }
            disabled={nextSamplerTunePreset(source.tune, coarse, direction) === null}
            onClick={() => {
              const next = nextSamplerTunePreset(storedTune(id), coarse, direction)
              if (next !== null) {
                setCoarse(next.semitones)
                void send({ tune: next.tune })
              }
            }}
          >
            {direction === "previous" ? "Previous" : "Next"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            disabled={nextSamplerTuneScale(source.tune, coarse, factor) === null}
            onClick={() => {
              const next = nextSamplerTuneScale(storedTune(id), coarse, factor)
              if (next !== null) {
                setCoarse(next.semitones)
                void send({ tune: next.tune })
              }
            }}
          >
            {factor === "half" ? "Half" : "Double"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {FINE_TUNE_PRESETS.map(({ label, cents: preset }) => (
          <Button
            key={label}
            variant="outline"
            size="sm"
            disabled={nextSamplerFine(source.tune, coarse, preset) === null}
            onClick={() => {
              const next = nextSamplerFine(storedTune(id), coarse, preset)
              if (next !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { tune: next },
                })
              }
            }}
          >
            {label}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["previous", "next"] as const).map((direction) => (
          <Button
            key={direction}
            variant="outline"
            size="sm"
            aria-label={
              direction === "previous"
                ? "Choose the previous sampler fine tune"
                : "Choose the next sampler fine tune"
            }
            disabled={
              nextSamplerFinePreset(source.tune, coarse, direction) === null
            }
            onClick={() => {
              const next = nextSamplerFinePreset(storedTune(id), coarse, direction)
              if (next !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { tune: next },
                })
              }
            }}
          >
            {direction === "previous" ? "Previous" : "Next"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            aria-label={
              factor === "half"
                ? "Halve fine tune cents"
                : "Double fine tune cents"
            }
            disabled={nextSamplerFineScale(source.tune, coarse, factor) === null}
            onClick={() => {
              const next = nextSamplerFineScale(storedTune(id), coarse, factor)
              if (next !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { tune: next },
                })
              }
            }}
          >
            {factor === "half" ? "Halve cents" : "Double cents"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {SAMPLE_GAIN_PRESETS.map(({ label, gain: preset }) => (
          <Button
            key={label}
            variant="outline"
            size="sm"
            disabled={nextSampleGain(source.gain, preset) === null}
            onClick={() => {
              const latest = findChannel(id)?.source
              if (latest?.type !== "sampler") return
              if (nextSampleGain(latest.gain, preset) !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { gain: preset },
                })
              }
            }}
          >
            {label}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["previous", "next"] as const).map((direction) => (
          <Button
            key={direction}
            variant="outline"
            size="sm"
            aria-label={
              direction === "previous"
                ? "Choose the previous sample gain"
                : "Choose the next sample gain"
            }
            disabled={nextSampleGainPreset(source.gain, direction) === null}
            onClick={() => {
              const latest = findChannel(id)?.source
              if (latest?.type !== "sampler") return
              const next = nextSampleGainPreset(latest.gain, direction)
              if (next !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { gain: next },
                })
              }
            }}
          >
            {direction === "previous" ? "Previous" : "Next"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            disabled={nextSampleGainScale(source.gain, factor) === null}
            onClick={() => {
              const latest = findChannel(id)?.source
              if (latest?.type !== "sampler") return
              const next = nextSampleGainScale(latest.gain, factor)
              if (next !== null) {
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { gain: next },
                })
              }
            }}
          >
            {factor === "half" ? "Half" : "Double"}
          </Button>
        ))}
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
        <div className="mt-1 flex flex-wrap gap-1">
          {CUT_GROUP_PRESETS.map(({ label, cutGroup: preset }) => (
            <Button
              key={label}
              variant="outline"
              size="sm"
              disabled={nextCutGroup(source.cutGroup, preset) === null}
              onClick={() => {
                const latest = findChannel(id)?.source
                if (latest?.type !== "sampler") return
                if (nextCutGroup(latest.cutGroup, preset) !== null) {
                  void dispatch({
                    type: "updateSampler",
                    id,
                    patch: { cutGroup: preset },
                  })
                }
              }}
            >
              {label}
            </Button>
          ))}
        </div>
        <div className="mt-1 flex flex-wrap gap-1">
          {(["previous", "next"] as const).map((direction) => (
            <Button
              key={direction}
              variant="outline"
              size="sm"
              aria-label={
                direction === "previous"
                  ? "Choose the previous cut group preset"
                  : "Choose the next cut group preset"
              }
              disabled={nextCutGroupPreset(source.cutGroup, direction) === null}
              onClick={() => {
                const latest = findChannel(id)?.source
                if (latest?.type !== "sampler") return
                const next = nextCutGroupPreset(latest.cutGroup, direction)
                if (next === null) return
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { cutGroup: next },
                })
              }}
            >
              {direction === "previous" ? "Previous" : "Next"}
            </Button>
          ))}
        </div>
      </div>
    </Section>
  )
}
