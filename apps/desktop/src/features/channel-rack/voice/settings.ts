/** Persistent types generated from windfall-project. */
import type {
  EchoTime,
  ModulationEnvelope,
  ChannelVoiceSettings,
} from "@/bindings"
export type {
  ChannelNoteDivision as NoteDivision,
  ArpeggiatorMode,
  EchoTime,
  ChannelLfoShape as LfoShape,
  LfoTarget,
  ArpeggiatorSettings,
  NoteEchoSettings,
  PolyphonySettings,
  ModulationEnvelope,
  ChannelLfoSettings,
  ChannelEnvelopes,
  ChannelVoiceSettings,
} from "@/bindings"

export function createDefaultChannelVoiceSettings(): ChannelVoiceSettings {
  const envelope = (): ModulationEnvelope => ({
    enabled: false,
    attackMs: 2,
    decayMs: 200,
    sustain: 0.8,
    releaseMs: 150,
    depth: 0,
  })
  return {
    arpeggiator: {
      mode: "off",
      rate: "sixteenth",
      gate: 0.75,
      rangeOctaves: 1,
    },
    echo: {
      enabled: false,
      time: { unit: "milliseconds", ms: 250 },
      feedback: 0.5,
      pitchSemitones: 0,
      repeats: 3,
    },
    polyphony: { maxVoices: 32, monoLegato: false, portamentoMs: 0 },
    envelopes: {
      filter: envelope(),
      pitch: envelope(),
      pan: envelope(),
      lfo: {
        enabled: false,
        shape: "sine",
        target: "filter",
        rateHz: 5,
        depth: 0,
      },
    },
  }
}

function finite(value: number, low: number, high: number, fallback: number) {
  return Number.isFinite(value)
    ? Math.min(high, Math.max(low, value))
    : fallback
}

export function assertNever(value: never): never {
  throw new Error(`Unexpected channel voice value: ${String(value)}`)
}

/** Match Rust ranges and ensure integer fields can deserialize into u8/i8. */
export function sanitizeChannelVoiceSettings(
  value: ChannelVoiceSettings
): ChannelVoiceSettings {
  function envelope(
    value: ModulationEnvelope,
    depth: number
  ): ModulationEnvelope {
    return {
      ...value,
      attackMs: finite(value.attackMs, 0, 10_000, 2),
      decayMs: finite(value.decayMs, 1, 10_000, 200),
      sustain: finite(value.sustain, 0, 1, 0.8),
      releaseMs: finite(value.releaseMs, 1, 10_000, 150),
      depth: finite(value.depth, -depth, depth, 0),
    }
  }
  function time(value: EchoTime): EchoTime {
    switch (value.unit) {
      case "milliseconds":
        return { unit: value.unit, ms: finite(value.ms, 1, 60_000, 250) }
      case "division":
        return { ...value }
      default:
        return assertNever(value)
    }
  }
  return {
    arpeggiator: {
      ...value.arpeggiator,
      gate: finite(value.arpeggiator.gate, 0, 1, 0.75),
      rangeOctaves: Math.trunc(finite(value.arpeggiator.rangeOctaves, 1, 4, 1)),
    },
    echo: {
      ...value.echo,
      time: time(value.echo.time),
      feedback: finite(value.echo.feedback, 0, 0.95, 0.5),
      pitchSemitones: Math.trunc(finite(value.echo.pitchSemitones, -48, 48, 0)),
      repeats: Math.trunc(finite(value.echo.repeats, 0, 8, 3)),
    },
    polyphony: {
      ...value.polyphony,
      maxVoices: Math.trunc(finite(value.polyphony.maxVoices, 1, 32, 32)),
      portamentoMs: finite(value.polyphony.portamentoMs, 0, 60_000, 0),
    },
    envelopes: {
      filter: envelope(value.envelopes.filter, 8),
      pitch: envelope(value.envelopes.pitch, 2400),
      pan: envelope(value.envelopes.pan, 1),
      lfo: {
        ...value.envelopes.lfo,
        rateHz: finite(value.envelopes.lfo.rateHz, 0.01, 30, 5),
        depth: finite(value.envelopes.lfo.depth, -1, 1, 0),
      },
    },
  }
}
