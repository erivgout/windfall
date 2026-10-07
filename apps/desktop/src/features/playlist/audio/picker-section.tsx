import { AudioWave01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { memo } from "react"

import type { SampleAsset } from "@/bindings"
import { ContextActions, type ContextItem } from "@/components/context-actions"
import { useHint } from "@/lib/store/hint"
import { useProjectStore } from "@/lib/store/project"

import { PICKER_ROW, PickerEmpty, PickerSection } from "../picker-section"
import { songTick } from "../ops"
import { useAudioClipCounts } from "../selectors"
import { usePlaylistStore } from "../store"
import { placeSampleClips } from "./ops"

function used(clips: number): string {
  return clips === 0
    ? "not on the timeline yet"
    : `on the timeline ${clips === 1 ? "once" : `${clips} times`}`
}

/** Puts a clip of the sample at the song position, on a new track. */
async function addAtSongPosition(sample: SampleAsset) {
  const row = useProjectStore.getState().project.playlist.tracks.length
  const created = await placeSampleClips(
    sample,
    [
      {
        row,
        start: Math.max(0, Math.round(songTick())),
        length: 1,
        offset: 0,
        muted: false,
        content: {
          type: "audio",
          sample: sample.id,
          mixerTrack: 0,
          gain: 1,
          pan: 0,
          fadeIn: 0,
          fadeOut: 0,
          reverse: false,
          pitch: 0,
        },
      },
    ],
    "Add audio clip"
  )
  if (created && created.length > 0) usePlaylistStore.getState().select(created)
}

const SampleRow = memo(function SampleRow({
  sample,
  selected,
  clips,
}: {
  sample: SampleAsset
  selected: boolean
  clips: number
}) {
  const setBrush = usePlaylistStore((state) => state.setBrush)
  const hint = useHint(
    selected
      ? `${sample.name} is what Draw and Paint place, as an audio clip. It is ${used(clips)}`
      : `Click to place ${sample.name} as an audio clip with Draw and Paint. It is ${used(clips)}`
  )
  const pick = () => setBrush({ type: "audio", sample: sample.id })
  const menu: ContextItem[] = [
    { title: "Place with Draw and Paint", checked: selected, run: pick },
    {
      title: "Add to playlist at the song position",
      run: () => addAtSongPosition(sample),
    },
  ]

  return (
    <ContextActions items={menu}>
      <button
        type="button"
        aria-pressed={selected}
        data-sample={sample.id}
        onClick={pick}
        className={PICKER_ROW}
        {...hint}
      >
        <HugeiconsIcon
          icon={AudioWave01Icon}
          strokeWidth={2}
          className="size-3.5 shrink-0 text-muted-foreground"
        />
        <span className="min-w-0 flex-1 truncate group-aria-pressed:font-medium">
          {sample.name}
        </span>
        {clips > 0 && (
          <span className="shrink-0 font-readout text-[0.625rem] text-muted-foreground">
            ×{clips}
          </span>
        )}
      </button>
    </ContextActions>
  )
})

/**
 * The sounds of the project, to place on the timeline as audio clips. One
 * that is picked becomes the brush, as a pattern does.
 */
export function AudioSection() {
  const samples = useProjectStore((state) => state.project.samples)
  const brush = usePlaylistStore((state) => state.brush)
  const counts = useAudioClipCounts()

  return (
    <PickerSection title="Audio">
      {samples.length === 0 ? (
        <PickerEmpty>
          Drag a sound from the browser onto the timeline to make an audio clip.
        </PickerEmpty>
      ) : (
        <div role="group" aria-label="Sound to place" className="flex flex-col">
          {samples.map((sample) => (
            <SampleRow
              key={sample.id}
              sample={sample}
              selected={brush.type === "audio" && brush.sample === sample.id}
              clips={counts.get(sample.id) ?? 0}
            />
          ))}
        </div>
      )}
    </PickerSection>
  )
}
