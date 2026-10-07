import { instrumentDescriptor } from "@/features/params"
import type { InstrumentChannel } from "@/lib/channel-source"
import { colorToCss } from "@/lib/units"

import { SoundMenu } from "../synth/sound-menu"
import { SynthEditor } from "../synth/synth-editor"
import { Section } from "./parts"

/**
 * The settings of an instrument channel: the instrument's own panel under
 * its name, with the list of starting sounds beside it.
 */
export function InstrumentSection({ channel }: { channel: InstrumentChannel }) {
  const { params } = channel.source
  return (
    <Section
      title={instrumentDescriptor(params.type).name}
      aside={<SoundMenu params={params} />}
    >
      <SynthEditor
        channel={channel.id}
        params={params}
        color={colorToCss(channel.color)}
      />
    </Section>
  )
}
