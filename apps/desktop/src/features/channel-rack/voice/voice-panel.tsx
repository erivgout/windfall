import { useId, useRef, useState } from "react"

import { Knob, msUnit, percentUnit, powerScale } from "@/components/audio"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { Switch } from "@/components/ui/switch"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"

import {
  createDefaultChannelVoiceSettings,
  sanitizeChannelVoiceSettings,
  type ChannelVoiceSettings,
  type LfoTarget,
  type ModulationEnvelope,
  type NoteDivision,
} from "./settings"

const TIME_SCALE = powerScale(3)
const DIVISIONS: readonly { value: NoteDivision; label: string }[] = [
  { value: "quarter", label: "1/4" },
  { value: "eighth", label: "1/8" },
  { value: "sixteenth", label: "1/16" },
  { value: "thirtySecond", label: "1/32" },
]

function Choices<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: T
  options: readonly { value: T; label: string }[]
  onChange(value: T): void
}) {
  const id = useId()
  return (
    <Field>
      <FieldLabel id={id}>{label}</FieldLabel>
      <ToggleGroup
        aria-labelledby={id}
        value={[value]}
        variant="outline"
        size="sm"
        spacing={0}
        className="flex-wrap"
        onValueChange={(values) => {
          const selected = options.find((option) => option.value === values[0])
          if (selected) onChange(selected.value)
        }}
      >
        {options.map((option) => (
          <ToggleGroupItem key={option.value} value={option.value}>
            {option.label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </Field>
  )
}

function Enabled({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange(value: boolean): void
}) {
  const id = useId()
  return (
    <Field orientation="horizontal">
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Switch id={id} size="sm" checked={checked} onCheckedChange={onChange} />
    </Field>
  )
}

function EnvelopeControls({
  target,
  value,
  onChange,
}: {
  target: LfoTarget
  value: ModulationEnvelope
  onChange(value: ModulationEnvelope): void
}) {
  const title = `${target[0].toUpperCase()}${target.slice(1)} envelope`
  const maxDepth = target === "filter" ? 8 : target === "pitch" ? 2400 : 1
  const depthUnit =
    target === "filter" ? "oct" : target === "pitch" ? "ct" : "pan"
  const set = (patch: Partial<ModulationEnvelope>) =>
    onChange({ ...value, ...patch })
  return (
    <FieldSet className="border-b px-2.5 py-2">
      <FieldLegend>{title}</FieldLegend>
      <FieldGroup>
        <Enabled
          label={`Enable ${target} envelope`}
          checked={value.enabled}
          onChange={(enabled) => set({ enabled })}
        />
        <FieldGroup className="grid grid-cols-2 justify-items-center gap-2 @min-[22rem]:grid-cols-5">
          <Field>
            <Knob
              label="Attack"
              aria-label={`${title} attack`}
              value={value.attackMs}
              min={0}
              max={10_000}
              defaultValue={2}
              scale={TIME_SCALE}
              showValue
              {...msUnit}
              onValueChange={(attackMs) => set({ attackMs })}
            />
          </Field>
          <Field>
            <Knob
              label="Decay"
              aria-label={`${title} decay`}
              value={value.decayMs}
              min={1}
              max={10_000}
              defaultValue={200}
              scale={TIME_SCALE}
              showValue
              {...msUnit}
              onValueChange={(decayMs) => set({ decayMs })}
            />
          </Field>
          <Field>
            <Knob
              label="Sustain"
              aria-label={`${title} sustain`}
              value={value.sustain}
              min={0}
              max={1}
              defaultValue={0.8}
              showValue
              {...percentUnit}
              onValueChange={(sustain) => set({ sustain })}
            />
          </Field>
          <Field>
            <Knob
              label="Release"
              aria-label={`${title} release`}
              value={value.releaseMs}
              min={1}
              max={10_000}
              defaultValue={150}
              scale={TIME_SCALE}
              showValue
              {...msUnit}
              onValueChange={(releaseMs) => set({ releaseMs })}
            />
          </Field>
          <Field>
            <Knob
              label="Depth"
              aria-label={`${title} depth`}
              value={value.depth}
              min={-maxDepth}
              max={maxDepth}
              defaultValue={0}
              bipolar
              showValue
              format={(depth) =>
                `${depth.toFixed(target === "pitch" ? 0 : 2)} ${depthUnit}`
              }
              onValueChange={(depth) => set({ depth })}
            />
          </Field>
        </FieldGroup>
      </FieldGroup>
    </FieldSet>
  )
}

export interface ChannelVoicePanelProps {
  /** Initial local draft. Remount with a channel/revision key to replace it. */
  initialSettings?: ChannelVoiceSettings
  onChange(settings: ChannelVoiceSettings): void
}

/** Local editor: the parent owns persistence and engine application. */
export function ChannelVoicePanel({
  initialSettings,
  onChange,
}: ChannelVoicePanelProps) {
  const [settings, setSettings] = useState(() =>
    sanitizeChannelVoiceSettings(
      initialSettings ?? createDefaultChannelVoiceSettings()
    )
  )
  // Keep rapid edits coherent before React renders; onChange stays outside a
  // state updater so Strict Mode cannot duplicate external side effects.
  const draft = useRef(settings)
  function update(patch: Partial<ChannelVoiceSettings>) {
    const next = sanitizeChannelVoiceSettings({ ...draft.current, ...patch })
    draft.current = next
    setSettings(next)
    onChange(next)
  }
  const arp = settings.arpeggiator
  const echo = settings.echo
  const poly = settings.polyphony
  const lfo = settings.envelopes.lfo
  const setArp = (patch: Partial<typeof arp>) =>
    update({ arpeggiator: { ...draft.current.arpeggiator, ...patch } })
  const setEcho = (patch: Partial<typeof echo>) =>
    update({ echo: { ...draft.current.echo, ...patch } })
  const setPoly = (patch: Partial<typeof poly>) =>
    update({ polyphony: { ...draft.current.polyphony, ...patch } })
  const setLfo = (patch: Partial<typeof lfo>) =>
    update({
      envelopes: {
        ...draft.current.envelopes,
        lfo: { ...draft.current.envelopes.lfo, ...patch },
      },
    })

  return (
    <div
      aria-label="Channel voice settings"
      role="group"
      className="@container flex min-w-0 flex-col text-xs"
    >
      <FieldSet className="border-b px-2.5 py-2">
        <FieldLegend>Arpeggiator</FieldLegend>
        <FieldGroup>
          <Choices
            label="Arpeggiator mode"
            value={arp.mode}
            options={[
              { value: "off", label: "Off" },
              { value: "up", label: "Up" },
              { value: "down", label: "Down" },
              { value: "upDown", label: "Up-down" },
              { value: "asPlayed", label: "As played" },
            ]}
            onChange={(mode) => setArp({ mode })}
          />
          <Choices
            label="Arpeggiator rate"
            value={arp.rate}
            options={DIVISIONS}
            onChange={(rate) => setArp({ rate })}
          />
          <FieldGroup className="grid grid-cols-2 justify-items-center gap-2">
            <Field>
              <Knob
                label="Gate"
                aria-label="Arpeggiator gate"
                value={arp.gate}
                min={0}
                max={1}
                defaultValue={0.75}
                showValue
                {...percentUnit}
                onValueChange={(gate) => setArp({ gate })}
              />
            </Field>
            <Field>
              <Knob
                label="Octaves"
                aria-label="Arpeggiator octave range"
                value={arp.rangeOctaves}
                min={1}
                max={4}
                step={1}
                defaultValue={1}
                showValue
                onValueChange={(rangeOctaves) => setArp({ rangeOctaves })}
              />
            </Field>
          </FieldGroup>
        </FieldGroup>
      </FieldSet>
      <FieldSet className="border-b px-2.5 py-2">
        <FieldLegend>Note echo</FieldLegend>
        <FieldGroup>
          <Enabled
            label="Enable note echo"
            checked={echo.enabled}
            onChange={(enabled) => setEcho({ enabled })}
          />
          <Choices
            label="Echo timing"
            value={echo.time.unit}
            options={[
              { value: "milliseconds", label: "Milliseconds" },
              { value: "division", label: "Tempo" },
            ]}
            onChange={(unit) => {
              if (unit !== draft.current.echo.time.unit)
                setEcho({
                  time:
                    unit === "milliseconds"
                      ? { unit, ms: 250 }
                      : { unit, division: "eighth" },
                })
            }}
          />
          {echo.time.unit === "division" ? (
            <Choices
              label="Echo note division"
              value={echo.time.division}
              options={DIVISIONS}
              onChange={(division) =>
                setEcho({ time: { unit: "division", division } })
              }
            />
          ) : (
            <Field>
              <Knob
                label="Delay"
                aria-label="Echo delay time"
                value={echo.time.ms}
                min={1}
                max={60_000}
                defaultValue={250}
                scale={TIME_SCALE}
                showValue
                {...msUnit}
                onValueChange={(ms) =>
                  setEcho({ time: { unit: "milliseconds", ms } })
                }
              />
            </Field>
          )}
          <FieldGroup className="grid grid-cols-3 justify-items-center gap-2">
            <Field>
              <Knob
                label="Feedback"
                aria-label="Echo feedback"
                value={echo.feedback}
                min={0}
                max={0.95}
                defaultValue={0.5}
                showValue
                {...percentUnit}
                onValueChange={(feedback) => setEcho({ feedback })}
              />
            </Field>
            <Field>
              <Knob
                label="Pitch"
                aria-label="Echo pitch offset"
                value={echo.pitchSemitones}
                min={-48}
                max={48}
                step={1}
                defaultValue={0}
                bipolar
                showValue
                format={(pitch) => `${pitch} st`}
                onValueChange={(pitchSemitones) => setEcho({ pitchSemitones })}
              />
            </Field>
            <Field>
              <Knob
                label="Repeats"
                aria-label="Echo repeat count"
                value={echo.repeats}
                min={0}
                max={8}
                step={1}
                defaultValue={3}
                showValue
                onValueChange={(repeats) => setEcho({ repeats })}
              />
            </Field>
          </FieldGroup>
          <FieldDescription>
            Each repeat adds the pitch offset and fades by feedback. Very quiet
            repeats stop early.
          </FieldDescription>
        </FieldGroup>
      </FieldSet>
      <FieldSet className="border-b px-2.5 py-2">
        <FieldLegend>Polyphony and glide</FieldLegend>
        <FieldGroup>
          <Enabled
            label="Mono legato"
            checked={poly.monoLegato}
            onChange={(monoLegato) => setPoly({ monoLegato })}
          />
          <FieldGroup className="grid grid-cols-2 justify-items-center gap-2">
            <Field>
              <Knob
                label="Voices"
                aria-label="Maximum channel voices"
                value={poly.maxVoices}
                min={1}
                max={32}
                step={1}
                defaultValue={32}
                showValue
                onValueChange={(maxVoices) => setPoly({ maxVoices })}
              />
            </Field>
            <Field>
              <Knob
                label="Portamento"
                aria-label="Portamento time"
                value={poly.portamentoMs}
                min={0}
                max={60_000}
                defaultValue={0}
                scale={TIME_SCALE}
                showValue
                {...msUnit}
                onValueChange={(portamentoMs) => setPoly({ portamentoMs })}
              />
            </Field>
          </FieldGroup>
          <FieldDescription>
            Mono legato keeps one voice and preserves envelopes between
            overlapping notes. Portamento time covers 63% of the pitch change.
          </FieldDescription>
        </FieldGroup>
      </FieldSet>
      {(["filter", "pitch", "pan"] as const).map((target) => (
        <EnvelopeControls
          key={target}
          target={target}
          value={settings.envelopes[target]}
          onChange={(value) =>
            update({
              envelopes: { ...draft.current.envelopes, [target]: value },
            })
          }
        />
      ))}
      <FieldSet className="border-b px-2.5 py-2">
        <FieldLegend>Voice LFO</FieldLegend>
        <FieldGroup>
          <Enabled
            label="Enable voice LFO"
            checked={lfo.enabled}
            onChange={(enabled) => setLfo({ enabled })}
          />
          <Choices
            label="LFO waveform"
            value={lfo.shape}
            options={[
              { value: "sine", label: "Sine" },
              { value: "triangle", label: "Triangle" },
              { value: "square", label: "Square" },
            ]}
            onChange={(shape) => setLfo({ shape })}
          />
          <Choices
            label="LFO target"
            value={lfo.target}
            options={[
              { value: "filter", label: "Filter" },
              { value: "pitch", label: "Pitch" },
              { value: "pan", label: "Pan" },
            ]}
            onChange={(target) => setLfo({ target })}
          />
          <FieldGroup className="grid grid-cols-2 justify-items-center gap-2">
            <Field>
              <Knob
                label="Rate"
                aria-label="LFO rate"
                value={lfo.rateHz}
                min={0.01}
                max={30}
                defaultValue={5}
                scale="log"
                showValue
                format={(rate) => `${rate.toFixed(2)} Hz`}
                onValueChange={(rateHz) => setLfo({ rateHz })}
              />
            </Field>
            <Field>
              <Knob
                label="Depth"
                aria-label="LFO depth"
                value={lfo.depth}
                min={-1}
                max={1}
                defaultValue={0}
                bipolar
                showValue
                {...percentUnit}
                onValueChange={(depth) => setLfo({ depth })}
              />
            </Field>
          </FieldGroup>
          <FieldDescription>
            Full depth: 8 filter octaves, 2400 pitch cents, or full pan travel.
            Negative depth reverses the motion.
          </FieldDescription>
        </FieldGroup>
      </FieldSet>
    </div>
  )
}
