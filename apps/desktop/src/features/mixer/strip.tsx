import { ArrowRight02Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { memo } from "react"

import type { MixerTrack, TrackId } from "@/bindings"
import { MuteSolo, PanControl, ToggleLed } from "@/components/audio"
import { runAction } from "@/lib/actions"
import {
  useEngineStore,
  useHint,
  useMixerTrack,
  useMixerTrackIndex,
  useProjectStore,
  useUiStore,
} from "@/lib/store"
import { MASTER_TRACK } from "@/lib/units"
import { cn } from "@/lib/utils"

import { ChannelChips } from "./channel-chips"
import { SEND_ROW_HEIGHT, type StripMode } from "./layout"
import { LevelSection, type LevelLayout } from "./level-section"
import { clampPan, patchTrack } from "./operations"
import { OutputSelect } from "./output-select"
import { heardTracks, type Audibility } from "./routing"
import { RoutingButton } from "./routing-popover"
import { AddSendMenu, SendList } from "./sends"
import { StripHeader } from "./strip-header"
import { useGestureValue } from "./use-gesture-value"

/** Fades the parts of a strip that solo on another track has silenced. */
const DIMMED =
  "transition-opacity group-data-[audible=silenced]/strip:opacity-35"

function PanKnob({
  track,
  showValue,
}: {
  track: MixerTrack
  showValue: boolean
}) {
  const pan = useGestureValue(
    track.pan,
    (value) => ({
      type: "updateMixerTrack",
      id: track.id,
      patch: { pan: value },
    }),
    clampPan
  )
  const hint = useHint("Pan. Drag, or double-click to center")
  return (
    <PanControl
      size="sm"
      showValue={showValue}
      aria-label={`${track.name} pan`}
      className={cn(
        DIMMED,
        "[&_[data-slot=knob-value]]:font-readout [&_[data-slot=knob-value]]:text-[9px] [&_[data-slot=knob-value]]:text-muted-foreground"
      )}
      {...pan}
      {...hint}
    />
  )
}

function MuteButtons({ track, small }: { track: MixerTrack; small: boolean }) {
  const hint = useHint(
    track.id === MASTER_TRACK
      ? "Mute silences everything"
      : "Mute silences this track. Solo leaves only it and what it needs to be heard"
  )
  const size = small ? "sm" : "md"
  if (track.id === MASTER_TRACK) {
    return (
      <ToggleLed
        size={size}
        pressed={track.muted}
        color="var(--wf-mute, var(--wf-meter-mid))"
        aria-label="Mute"
        onPressedChange={(muted) => void patchTrack(track.id, { muted })}
        {...hint}
      >
        M
      </ToggleLed>
    )
  }
  return (
    <MuteSolo
      size={size}
      muted={track.muted}
      solo={track.solo}
      onMutedChange={(muted) => void patchTrack(track.id, { muted })}
      onSoloChange={(solo) => void patchTrack(track.id, { solo })}
      {...hint}
    />
  )
}

/** Where the master goes: the sound card. Opens the audio settings. */
function MasterOutput() {
  const device = useEngineStore((state) =>
    state.status?.running ? state.status.device : null
  )
  const hint = useHint(
    "The master plays to the audio output. Click to choose a device"
  )
  return (
    <button
      type="button"
      data-slot="track-output"
      aria-label={`Output: ${device ?? "no audio device"}`}
      className="flex h-[18px] w-full min-w-0 items-center gap-0.5 rounded-[3px] pr-1 pl-0.5 text-[10px] leading-none text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
      onClick={() => void runAction("options.settings")}
      {...hint}
    >
      <HugeiconsIcon
        icon={ArrowRight02Icon}
        strokeWidth={2}
        className="size-3 shrink-0"
      />
      <span className="truncate">{device ?? "No device"}</span>
    </button>
  )
}

/** The output selector and the sends, inline. Every strip is equally tall. */
function Routing({ track, sendRows }: { track: MixerTrack; sendRows: number }) {
  const master = track.id === MASTER_TRACK
  return (
    <div
      data-slot="track-routing-inline"
      className={cn("flex shrink-0 flex-col gap-0.5 px-1", DIMMED)}
    >
      {master ? (
        <MasterOutput />
      ) : (
        <OutputSelect id={track.id} output={track.output} className="w-full" />
      )}
      <div style={{ height: sendRows * SEND_ROW_HEIGHT }}>
        {!master && (
          <SendList id={track.id} sends={track.sends} className="max-h-full" />
        )}
      </div>
      <div className="flex h-[18px]">
        {!master && <AddSendMenu id={track.id} />}
      </div>
    </div>
  )
}

function stripLabel(track: MixerTrack, index: number, audible: Audibility) {
  const place = track.id === MASTER_TRACK ? "master track" : `track ${index}`
  const state =
    audible === "muted"
      ? ", muted"
      : audible === "silenced"
        ? ", silenced by solo"
        : ""
  return `${track.name}, ${place}${state}`
}

const LEVEL_LAYOUT: Record<StripMode, LevelLayout> = {
  full: "tall",
  compact: "tall",
  tight: "short",
  flat: "flat",
  mini: "bare",
}

export type MixerStripProps = {
  id: TrackId
  mode: StripMode
  /** Send rows every strip keeps free in "full" mode. */
  sendRows: number
  /** The fader is too short to label every dB mark. */
  sparseScale: boolean
  /** The channel selected in the rack plays into this track. */
  linked: boolean
  /** False for a strip kept mounted while it is scrolled out of view. */
  metering: boolean
}

/**
 * One mixer track. It reads its own track from the store, so moving a fader
 * renders this strip and no other.
 */
export const MixerStrip = memo(function MixerStrip({
  id,
  mode,
  sendRows,
  sparseScale,
  linked,
  metering,
}: MixerStripProps) {
  const track = useMixerTrack(id)
  const index = useMixerTrackIndex(id)
  const selected = useUiStore((state) => state.selectedTrack === id)
  const heard = useProjectStore((state) =>
    heardTracks(state.project.mixer.tracks).has(id)
  )

  if (!track) return null

  const master = id === MASTER_TRACK
  const audible: Audibility = track.muted
    ? "muted"
    : heard
      ? "heard"
      : "silenced"
  // The fader lies across the strip.
  const low = mode === "flat" || mode === "mini"
  // The channel chips have a row of their own.
  const chips = mode === "full" || mode === "compact"

  function select() {
    const ui = useUiStore.getState()
    if (ui.selectedTrack !== id) ui.selectTrack(id)
  }

  return (
    <div
      role="group"
      aria-label={stripLabel(track, index, audible)}
      data-track={id}
      data-mode={mode}
      data-audible={audible}
      data-selected={selected || undefined}
      data-linked={linked ? "" : undefined}
      tabIndex={0}
      onPointerDownCapture={select}
      onFocus={select}
      className={cn(
        "group/strip relative flex h-full w-full flex-col border-r outline-none focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
        low ? "gap-0.5" : "gap-1.5 pb-1.5",
        "data-linked:bg-[color-mix(in_oklch,var(--wf-brand)_10%,transparent)]",
        "data-selected:bg-accent data-selected:shadow-[inset_0_-2px_0_var(--wf-brand)]"
      )}
    >
      <StripHeader
        id={id}
        name={track.name}
        color={track.color}
        number={master ? "M" : String(index)}
        master={master}
        trailing={
          mode !== "full" && (!master || !chips) ? (
            <RoutingButton track={track} withChannels={!chips} />
          ) : null
        }
      />
      {chips && (
        <div className={DIMMED}>
          <ChannelChips id={id} master={master} />
        </div>
      )}
      {mode === "full" && <Routing track={track} sendRows={sendRows} />}
      <div className="flex shrink-0 items-start justify-center gap-2">
        <PanKnob track={track} showValue={mode === "full"} />
        <div className="flex h-6 items-center">
          <MuteButtons track={track} small={low} />
        </div>
      </div>
      <div
        className={cn(
          "flex min-h-0 flex-1 flex-col group-data-[audible=muted]/strip:opacity-55",
          DIMMED
        )}
      >
        <LevelSection
          id={id}
          name={track.name}
          volume={track.volume}
          index={index}
          metering={metering}
          layout={LEVEL_LAYOUT[mode]}
          sparseScale={sparseScale}
          wide={master}
        />
      </div>
    </div>
  )
})
