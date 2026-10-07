import type {
  Channel,
  ChannelSource,
  InstrumentParams,
  SampleId,
} from "@/bindings"

/*
 * A channel is a sampler or an instrument. These narrow one to the other,
 * so code that needs a sample or an instrument's settings says so once.
 */

export type SamplerSource = Extract<ChannelSource, { type: "sampler" }>
export type InstrumentSource = Extract<ChannelSource, { type: "instrument" }>

export type SamplerChannel = Channel & { source: SamplerSource }
export type InstrumentChannel = Channel & { source: InstrumentSource }

export function isSamplerChannel(channel: Channel): channel is SamplerChannel {
  return channel.source.type === "sampler"
}

export function isInstrumentChannel(
  channel: Channel
): channel is InstrumentChannel {
  return channel.source.type === "instrument"
}

/** The sample a sampler plays. Null for an instrument and for an empty sampler. */
export function sourceSample(
  source: ChannelSource | undefined
): SampleId | null {
  return source?.type === "sampler" ? source.sample : null
}

/** The settings of the channel's instrument, or null for a sampler. */
export function instrumentParams(
  channel: Channel | undefined
): InstrumentParams | null {
  return channel?.source.type === "instrument" ? channel.source.params : null
}
