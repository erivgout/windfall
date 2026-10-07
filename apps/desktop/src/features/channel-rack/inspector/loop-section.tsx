import { formatPercent, Knob, percentUnit } from "@/components/audio"
import { Field, FieldDescription } from "@/components/ui/field"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import type { SamplerChannel } from "@/lib/channel-source"
import { useHint } from "@/lib/store/hint"
import { dispatch } from "@/lib/store/project"
import { clamp } from "@/lib/units"

import { findChannel } from "../channel-ops"
import { useGestureValue } from "../use-gesture-value"
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
      </div>
    </Section>
  )
}
