import type { LimiterParams } from "@/bindings"
import { ParamControl, ParamRow } from "@/features/params"

import type { EditorProps } from "../editor-props"
import { GainReductionMeter } from "../gain-reduction"
import { useSampleRate } from "../sample-rate"
import { formatLatency, lookaheadFrames } from "./latency"

/**
 * The limiter: how hard the signal is driven, the level nothing gets past,
 * and how the gain moves around a peak. It is the one effect that delays
 * its track, so it says by how much.
 */
export function LimiterEditor({ effect, bind }: EditorProps<LimiterParams>) {
  const sampleRate = useSampleRate()
  const frames = lookaheadFrames(bind.value("lookaheadMs"), sampleRate)

  return (
    <div data-slot="limiter-editor" className="flex max-w-[34rem] gap-3">
      <GainReductionMeter effect={effect} label="Limiter" className="h-28" />
      <div className="flex min-w-0 flex-1 flex-col justify-between gap-2">
        <ParamRow columns={4}>
          <ParamControl
            {...bind("inputGainDb")}
            label="Input"
            description="Turn it up to push the signal into the ceiling and make it louder"
          />
          <ParamControl
            {...bind("ceilingDb")}
            description="No output sample gets past this level"
          />
          <ParamControl
            {...bind("releaseMs")}
            description="How quickly the gain comes back after a peak. Short is louder but can distort bass"
          />
          <ParamControl
            {...bind("lookaheadMs")}
            description="How far ahead it sees a peak coming. Longer distorts less and delays the track more"
          />
        </ParamRow>
        <p
          data-slot="limiter-latency"
          className="text-[10px] leading-snug text-muted-foreground"
        >
          Looking ahead delays this track by{" "}
          <span className="font-readout text-foreground/85">
            {formatLatency(frames, sampleRate)}
          </span>
          . The engine delays every other track to match.
        </p>
      </div>
    </div>
  )
}
