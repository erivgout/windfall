import type { ComponentProps } from "react"

import { EnvelopeEditor, type EnvelopeValues } from "@/components/audio"

import type { ParamBinder } from "./use-param-binding"

type EnvelopeIds = Record<keyof EnvelopeValues, string>

type ParamEnvelopeProps = Omit<
  ComponentProps<typeof EnvelopeEditor>,
  | keyof EnvelopeValues
  | "onChange"
  | "onGestureStart"
  | "onGestureEnd"
  | "defaults"
  | "maxAttackMs"
  | "maxDecayMs"
  | "maxReleaseMs"
  | "disabled"
> & {
  bind: ParamBinder
  /**
   * The path the four settings share, such as `"ampEnvelope"` for
   * `ampEnvelope.attackMs`, `.decayMs`, `.sustain` and `.releaseMs`.
   */
  prefix: string
}

/**
 * The kit's envelope editor on four settings of a binding. It reads and
 * writes the same values as their knobs, so the two follow each other while
 * either is dragged, and one drag of a node is one undo step.
 */
export function ParamEnvelope({ bind, prefix, ...props }: ParamEnvelopeProps) {
  const ids: EnvelopeIds = {
    attackMs: `${prefix}.attackMs`,
    decayMs: `${prefix}.decayMs`,
    sustain: `${prefix}.sustain`,
    releaseMs: `${prefix}.releaseMs`,
  }
  const group = bind.group(prefix)

  return (
    <EnvelopeEditor
      // The envelopes of the built-in instruments: a rounded attack and
      // exponential falls. An editor of something else passes its own.
      curve="rounded"
      attackMs={bind.value(ids.attackMs)}
      decayMs={bind.value(ids.decayMs)}
      sustain={bind.value(ids.sustain)}
      releaseMs={bind.value(ids.releaseMs)}
      maxAttackMs={bind.info(ids.attackMs).max}
      maxDecayMs={bind.info(ids.decayMs).max}
      maxReleaseMs={bind.info(ids.releaseMs).max}
      defaults={{
        attackMs: bind.info(ids.attackMs).default,
        decayMs: bind.info(ids.decayMs).default,
        sustain: bind.info(ids.sustain).default,
        releaseMs: bind.info(ids.releaseMs).default,
      }}
      disabled={bind.disabled}
      onChange={(patch) => {
        for (const field of Object.keys(patch) as (keyof EnvelopeValues)[]) {
          const value = patch[field]
          if (value !== undefined) group.set(ids[field], value)
        }
      }}
      onGestureStart={group.onGestureStart}
      onGestureEnd={group.onGestureEnd}
      {...props}
    />
  )
}
