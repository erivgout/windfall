import type { ReverbParams } from "@/bindings"
import { ParamControl, ParamGroup, ParamRow } from "@/features/params"
import { cn } from "@/lib/utils"

import type { EditorProps } from "../editor-props"

/*
 * Docked beside the mixer's strips the editor is about 27rem wide and 15rem
 * tall. There the three groups stand side by side, each as wide as it has
 * columns of knobs (three, two and one), so nothing is below the fold.
 * Enlarged, and where the panel is too narrow for that, the groups are
 * equally wide with three knobs across, and wrap.
 */
const DOCKED = {
  root: "@min-[26rem]/editor:grid-cols-[3fr_2fr_1.15fr] @min-[26rem]/editor:gap-1.5 in-data-enlarged:grid-cols-[repeat(auto-fit,minmax(11.5rem,1fr))] in-data-enlarged:gap-2",
  two: "@min-[26rem]/editor:grid-cols-2! in-data-enlarged:grid-cols-3!",
  one: "@min-[26rem]/editor:grid-cols-1! in-data-enlarged:grid-cols-3!",
} as const

/** The reverb, in three groups: the room, its color, and how much of it. */
export function ReverbEditor({ bind }: EditorProps<ReverbParams>) {
  return (
    <div
      data-slot="reverb-editor"
      className={cn(
        "grid grid-cols-[repeat(auto-fit,minmax(11.5rem,1fr))] items-start gap-2",
        DOCKED.root
      )}
    >
      <ParamGroup title="Space">
        <ParamRow columns={3}>
          <ParamControl
            {...bind("size")}
            description="From a small room to a large hall. It sets how far apart the echoes are, not how long they last"
          />
          <ParamControl
            {...bind("decayS")}
            description="How long the tail takes to fade by 60 dB"
          />
          <ParamControl
            {...bind("preDelayMs")}
            description="The gap between the dry sound and the start of the reverb"
          />
          <ParamControl
            {...bind("diffusion")}
            description="Low lets single echoes through at the start, high gives a smooth wash"
          />
          <ParamControl
            {...bind("earlyLevel")}
            label="Early"
            description="Level of the first reflections, which say how close the walls are"
          />
        </ParamRow>
      </ParamGroup>
      <ParamGroup title="Tone">
        <ParamRow columns={3} className={DOCKED.two}>
          <ParamControl
            {...bind("damping")}
            description="How much faster the treble fades than the rest"
          />
          <ParamControl
            {...bind("modulation")}
            label="Mod"
            description="A slow shimmer in pitch that keeps the tail from ringing"
          />
          <ParamControl
            {...bind("lowCutHz")}
            description="Removes lows from what enters the reverb"
          />
          <ParamControl
            {...bind("highCutHz")}
            description="Removes highs from what enters the reverb"
          />
        </ParamRow>
      </ParamGroup>
      <ParamGroup title="Mix">
        <ParamRow columns={3} className={DOCKED.one}>
          <ParamControl
            {...bind("width")}
            description="Stereo width of the reverb, from mono to full"
          />
          <ParamControl
            {...bind("mix")}
            description="Balance between the dry sound and the reverb. Use 100% on a send track"
          />
        </ParamRow>
      </ParamGroup>
    </div>
  )
}
