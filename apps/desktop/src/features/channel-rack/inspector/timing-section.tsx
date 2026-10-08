import type { Channel, ChannelTiming } from "@/bindings"
import { Knob, NumberField, percentUnit } from "@/components/audio"
import { ActionButton } from "@/components/action-button"
import { useHint } from "@/lib/store/hint"
import { clamp, PPQ, MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

import { findChannel } from "../channel-ops"
import { useGestureValue } from "../use-gesture-value"
import { Section } from "./parts"

export const DEFAULT_CHANNEL_TIMING: ChannelTiming = { swingMix: 1, gateTicks: 0, shiftTicks: 0 }

export function TimingSection({ channel }: { channel: Channel }) {
  const { id } = channel
  const stored = channel.timing ?? DEFAULT_CHANNEL_TIMING
  // Each gesture reads the other controls at dispatch time so interleaved
  // changes do not overwrite the newest channel timing settings.
  const timing = () => findChannel(id)?.timing ?? DEFAULT_CHANNEL_TIMING
  const swing = useGestureValue(stored.swingMix, (value, dispatch) => dispatch({
    type: "updateChannel", id, patch: { timing: { ...timing(), swingMix: clamp(value, 0, 1) } },
  }))
  const gate = useGestureValue(stored.gateTicks, (value, dispatch) => dispatch({
    type: "updateChannel", id, patch: { timing: { ...timing(), gateTicks: clamp(Math.round(value), 0, MAX_PATTERN_STEPS * TICKS_PER_STEP) } },
  }))
  const shift = useGestureValue(stored.shiftTicks, (value, dispatch) => dispatch({
    type: "updateChannel", id, patch: { timing: { ...timing(), shiftTicks: clamp(Math.round(value), -PPQ, PPQ) } },
  }))
  const swingHint = useHint("Channel swing mix: how much of the global swing applies to this channel. 100% follows global swing; 0% stays straight")
  const gateHint = useHint("Gate caps the played duration after swing, in ticks. Zero keeps the original duration. Stored piano notes stay unchanged")
  const shiftHint = useHint("Shift moves played starts after swing, in ticks. Negative starts clamp to zero; starts reaching the pattern end do not play in that pass")
  return (
    <Section title="Note timing" aside={<ActionButton action="channel.resetTiming" variant="ghost" size="xs">Reset</ActionButton>}>
      <div className="flex items-center justify-between gap-2" {...swingHint}>
        <span>Swing mix</span>
        <Knob size="sm" aria-label="Channel swing mix" min={0} max={1} defaultValue={1} {...percentUnit} {...swing} />
      </div>
      <div className="flex items-center justify-between gap-2" {...gateHint}>
        <span>Gate</span>
        <NumberField size="sm" aria-label="Channel gate in ticks" min={0} max={MAX_PATTERN_STEPS * TICKS_PER_STEP} step={1} defaultValue={0} unit="ticks" {...gate} />
      </div>
      <div className="flex items-center justify-between gap-2" {...shiftHint}>
        <span>Shift</span>
        <NumberField size="sm" aria-label="Channel shift in ticks" min={-PPQ} max={PPQ} step={1} defaultValue={0} unit="ticks" {...shift} />
      </div>
      <p className="text-muted-foreground">{PPQ} ticks = one quarter note. Gate 0 keeps note lengths; filtering groups does not affect timing.</p>
    </Section>
  )
}
