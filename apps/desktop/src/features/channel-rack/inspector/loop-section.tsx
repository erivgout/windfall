import { formatPercent, Knob, percentUnit } from "@/components/audio"
import { Button } from "@/components/ui/button"
import { Field, FieldDescription } from "@/components/ui/field"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import type { SamplerChannel } from "@/lib/channel-source"
import { useHint } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { clamp } from "@/lib/units"

import { findChannel } from "../channel-ops"
import { useGestureValue } from "../use-gesture-value"
import { nextLoopCrossfadePreset } from "./crossfade-preset-step"
import {
  LOOP_CROSSFADE_PRESETS,
  nextLoopCrossfade,
} from "./loop-crossfade-presets"
import { nextLoopCrossfadeScale } from "./loop-crossfade-scale"
import { nextLoopLengthScale } from "./loop-length-scale"
import { nextLoopMode } from "./loop-mode-step"
import { nextLoopRegionPreset } from "./loop-region-preset-step"
import { LOOP_REGION_PRESETS, nextLoopRegion } from "./loop-region-presets"
import { nextLoopStartScale } from "./loop-start-scale"
import { Section } from "./parts"

/** A loop's points stay relative to the trimmed sample, in source order. */
export function LoopSection({ channel }: { channel: SamplerChannel }) {
  const { id, source } = channel
  const mode = source.loopMode ?? "off"
  const start = useGestureValue(source.loopStart ?? 0, (value, send) => {
    const current = findChannel(id)?.source
    const end = current?.type === "sampler" ? (current.loopEnd ?? 1) : 1
    return send({
      type: "updateSampler",
      id,
      patch: { loopStart: clamp(value, 0, Math.max(0, end - 0.000001)) },
    })
  })
  const end = useGestureValue(source.loopEnd ?? 1, (value, send) => {
    const current = findChannel(id)?.source
    const start = current?.type === "sampler" ? (current.loopStart ?? 0) : 0
    return send({
      type: "updateSampler",
      id,
      patch: { loopEnd: clamp(value, Math.min(1, start + 0.000001), 1) },
    })
  })
  const crossfade = useGestureValue(source.loopCrossfade ?? 0, (value, send) =>
    send({ type: "updateSampler", id, patch: { loopCrossfade: value } })
  )
  const crossfadeHint = useHint(
    "Loop crossfade: blends the loop seam during playback. Zero keeps the original loop"
  )
  const startHint = useHint(
    "Loop start: percentage of the trimmed sample, in source order"
  )
  const endHint = useHint(
    "Loop end: exclusive end within the trimmed sample. Short loops hold at least one frame"
  )

  return (
    <Section title="Loop">
      <Field>
        <ToggleGroup
          aria-label="Sample loop mode"
          size="sm"
          variant="outline"
          spacing={0}
          value={[mode]}
          onValueChange={(values) => {
            const loopMode = values[0]
            if (
              loopMode === "off" ||
              loopMode === "forward" ||
              loopMode === "pingPong"
            )
              void dispatch({ type: "updateSampler", id, patch: { loopMode } })
          }}
        >
          <ToggleGroupItem value="off">Off</ToggleGroupItem>
          <ToggleGroupItem value="forward">Forward</ToggleGroupItem>
          <ToggleGroupItem value="pingPong">Ping-pong</ToggleGroupItem>
        </ToggleGroup>
        <div className="flex flex-wrap gap-1">
          {(["previous", "next"] as const).map((direction) => (
            <Button
              key={direction}
              variant="outline"
              size="sm"
              aria-label={
                direction === "previous"
                  ? "Choose the previous loop mode"
                  : "Choose the next loop mode"
              }
              disabled={
                nextLoopMode(source.loopMode ?? "off", direction) === null
              }
              onClick={() => {
                const current = findChannel(id)?.source
                if (current?.type !== "sampler") return
                const next = nextLoopMode(current.loopMode ?? "off", direction)
                if (next !== null)
                  void dispatch({
                    type: "updateSampler",
                    id,
                    patch: { loopMode: next },
                  })
              }}
            >
              {direction === "previous" ? "Previous" : "Next"}
            </Button>
          ))}
        </div>
        <FieldDescription>
          Repeats while held, including release. Points are percentages of the
          trimmed sample.
        </FieldDescription>
      </Field>
      <div className="flex justify-center gap-4">
        <Knob
          label="Loop start"
          showValue
          min={0}
          max={1}
          step={0.001}
          defaultValue={0}
          {...percentUnit}
          format={(value) => formatPercent(value, 1)}
          {...start}
          {...startHint}
        />
        <Knob
          label="Loop end"
          showValue
          min={0}
          max={1}
          step={0.001}
          defaultValue={1}
          {...percentUnit}
          format={(value) => formatPercent(value, 1)}
          {...end}
          {...endHint}
        />
        <Knob
          label="Loop crossfade"
          showValue
          min={0}
          max={1}
          step={0.001}
          defaultValue={0}
          disabled={mode === "off"}
          {...percentUnit}
          format={(value) => formatPercent(value, 1)}
          {...crossfade}
          {...crossfadeHint}
        />
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {LOOP_CROSSFADE_PRESETS.map(({ label, crossfade: preset }) => (
          <Button
            key={label}
            variant="outline"
            size="sm"
            disabled={
              mode === "off" ||
              nextLoopCrossfade(source.loopCrossfade ?? 0, preset) === null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (
                current?.type !== "sampler" ||
                (current.loopMode ?? "off") === "off"
              )
                return
              if (
                nextLoopCrossfade(current.loopCrossfade ?? 0, preset) !== null
              )
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { loopCrossfade: preset },
                })
            }}
          >
            {label}
          </Button>
        ))}
      </div>
      <div className="flex flex-wrap gap-1">
        {(["previous", "next"] as const).map((direction) => (
          <Button
            key={direction}
            variant="outline"
            size="sm"
            aria-label={
              direction === "previous"
                ? "Choose the previous loop crossfade preset"
                : "Choose the next loop crossfade preset"
            }
            disabled={
              mode === "off" ||
              nextLoopCrossfadePreset(source.loopCrossfade ?? 0, direction) ===
                null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (
                current?.type !== "sampler" ||
                (current.loopMode ?? "off") === "off"
              )
                return
              const next = nextLoopCrossfadePreset(
                current.loopCrossfade ?? 0,
                direction
              )
              if (next === null) return
              void dispatch({
                type: "updateSampler",
                id,
                patch: { loopCrossfade: next },
              })
            }}
          >
            {direction === "previous" ? "Previous" : "Next"}
          </Button>
        ))}
      </div>
      <div className="flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            disabled={
              mode === "off" ||
              nextLoopCrossfadeScale(source.loopCrossfade ?? 0, factor) === null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (
                current?.type !== "sampler" ||
                (current.loopMode ?? "off") === "off"
              )
                return
              const next = nextLoopCrossfadeScale(
                current.loopCrossfade ?? 0,
                factor
              )
              if (next !== null)
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { loopCrossfade: next },
                })
            }}
          >
            {factor === "half" ? "Half" : "Double"}
          </Button>
        ))}
      </div>
      <div className="mt-1 flex flex-wrap gap-1">
        {LOOP_REGION_PRESETS.map((preset) => (
          <Button
            key={preset.label}
            variant="outline"
            size="sm"
            disabled={
              nextLoopRegion(
                source.loopStart ?? 0,
                source.loopEnd ?? 1,
                preset
              ) === null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (current?.type !== "sampler") return
              if (
                nextLoopRegion(
                  current.loopStart ?? 0,
                  current.loopEnd ?? 1,
                  preset
                ) !== null
              )
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { loopStart: preset.start, loopEnd: preset.end },
                })
            }}
          >
            {preset.label}
          </Button>
        ))}
      </div>
      <div className="flex flex-wrap gap-1">
        {(["previous", "next"] as const).map((direction) => (
          <Button
            key={direction}
            variant="outline"
            size="sm"
            aria-label={
              direction === "previous"
                ? "Choose the previous loop region preset"
                : "Choose the next loop region preset"
            }
            disabled={
              nextLoopRegionPreset(
                source.loopStart ?? 0,
                source.loopEnd ?? 1,
                direction
              ) === null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (current?.type !== "sampler") return
              const next = nextLoopRegionPreset(
                current.loopStart ?? 0,
                current.loopEnd ?? 1,
                direction
              )
              if (next === null) return
              void dispatch({
                type: "updateSampler",
                id,
                patch: { loopStart: next.start, loopEnd: next.end },
              })
            }}
          >
            {direction === "previous" ? "Previous" : "Next"}
          </Button>
        ))}
      </div>
      <div className="flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            aria-label={
              factor === "half" ? "Halve loop length" : "Double loop length"
            }
            disabled={
              nextLoopLengthScale(
                source.loopStart ?? 0,
                source.loopEnd ?? 1,
                factor
              ) === null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (current?.type !== "sampler") return
              const next = nextLoopLengthScale(
                current.loopStart ?? 0,
                current.loopEnd ?? 1,
                factor
              )
              if (next !== null)
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { loopEnd: next },
                })
            }}
          >
            {factor === "half" ? "Halve length" : "Double length"}
          </Button>
        ))}
      </div>
      <div className="flex flex-wrap gap-1">
        {(["half", "double"] as const).map((factor) => (
          <Button
            key={factor}
            variant="outline"
            size="sm"
            aria-label={
              factor === "half"
                ? "Halve loop length from the end"
                : "Double loop length from the end"
            }
            disabled={
              nextLoopStartScale(
                source.loopStart ?? 0,
                source.loopEnd ?? 1,
                factor
              ) === null
            }
            onClick={() => {
              const current = findChannel(id)?.source
              if (current?.type !== "sampler") return
              const next = nextLoopStartScale(
                current.loopStart ?? 0,
                current.loopEnd ?? 1,
                factor
              )
              if (next !== null)
                void dispatch({
                  type: "updateSampler",
                  id,
                  patch: { loopStart: next },
                })
            }}
          >
            {factor === "half" ? "Halve from end" : "Double from end"}
          </Button>
        ))}
      </div>
    </Section>
  )
}
