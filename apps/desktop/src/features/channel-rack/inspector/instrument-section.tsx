import { instrumentDescriptor } from "@/features/params"
import { PluginControls } from "@/features/plugins/controls"
import { usePluginBinding } from "@/features/plugins/store"
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
  const plugin = usePluginBinding({ type: "instrument", channel: channel.id })
  if (plugin)
    return (
      <Section title={plugin.name}>
        <PluginControls binding={plugin} />
      </Section>
    )
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
