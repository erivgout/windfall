import { PlayIcon, StopIcon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { useEffect, useRef, type ReactNode } from "react"

import { Waveform, type WaveformHandle } from "@/components/audio"
import { Button } from "@/components/ui/button"
import { openProjectPath } from "@/lib/flows/project"
import { useHint } from "@/lib/store/hint"
import { useChannel } from "@/lib/store/selectors"
import { useUiStore } from "@/lib/store/ui"

import { addToRack, replaceChannelSample } from "./commands"
import { formatChannels, formatDuration, formatSampleRate } from "./format"
import { previewEnded, requestPreview, stopPreview } from "./preview"
import { useBrowserStore, type Selection } from "./store"
import { extensionStart } from "./tree-model"

function PaneName({ selection }: { selection: Selection }) {
  const cut = extensionStart(selection.name, selection.kind)
  return (
    <p className="min-w-0 flex-1 truncate font-medium" title={selection.path}>
      {selection.name.slice(0, cut)}
      <span className="font-normal text-muted-foreground/70">
        {selection.name.slice(cut)}
      </span>
    </p>
  )
}

function channelColor(color: number): string {
  return `#${color.toString(16).padStart(6, "0")}`
}

/** Sets the selected channel's sample. Says which channel that is. */
function ReplaceButton({ path }: { path: string }) {
  const channel = useChannel(useUiStore((state) => state.selectedChannel))
  const explanation = channel
    ? `Make the channel ${channel.name} play this sound instead`
    : "Select a channel in the rack to replace its sample"
  const hint = useHint(explanation)

  return (
    // A disabled button takes no pointer events, so the wrapper explains it.
    <span className="flex min-w-0" title={explanation} {...hint}>
      <Button
        variant="outline"
        size="sm"
        disabled={!channel}
        aria-label={
          channel
            ? `Replace the sample of ${channel.name}`
            : "Replace selected channel's sample"
        }
        className="min-w-0 flex-1 justify-start gap-1.5"
        onClick={() => void replaceChannelSample(path)}
      >
        {channel ? (
          <>
            <span
              aria-hidden
              className="size-2 shrink-0 rounded-[2px]"
              style={{ backgroundColor: channelColor(channel.color) }}
            />
            <span className="truncate">Replace {channel.name}</span>
          </>
        ) : (
          <span className="truncate">Replace sample</span>
        )}
      </Button>
    </span>
  )
}

function Facts({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-4 items-center gap-2.5 overflow-hidden font-readout text-[10.5px] whitespace-nowrap text-muted-foreground">
      {children}
    </div>
  )
}

function SoundPane({ selection }: { selection: Selection }) {
  const { path } = selection
  const info = useBrowserStore((state) =>
    state.info?.path === path ? state.info : null
  )
  const startedAt = useBrowserStore((state) =>
    state.playing?.path === path ? state.playing.startedAt : null
  )
  const playError = useBrowserStore((state) =>
    state.previewError?.path === path ? state.previewError.message : null
  )
  const waveform = useRef<WaveformHandle>(null)
  const duration = info?.status === "ready" ? info.info.durationSecs : null

  // The engine does not report where the preview is, so the playhead runs
  // on the clock from the moment the engine took the sound. It is moved
  // without rendering, like every other realtime value.
  useEffect(() => {
    const handle = waveform.current
    if (startedAt === null || duration === null || !handle) return undefined
    let frame = 0
    const tick = () => {
      const position =
        (performance.now() - startedAt) / 1000 / Math.max(duration, 0.001)
      if (position >= 1) {
        handle.setPlayhead(null)
        previewEnded(path)
        return
      }
      handle.setPlayhead(position)
      frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => {
      cancelAnimationFrame(frame)
      handle.setPlayhead(null)
    }
  }, [startedAt, duration, path])

  const playing = startedAt !== null

  return (
    <>
      <div className="flex h-6 items-center gap-1">
        <PaneName selection={selection} />
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={playing ? "Stop preview" : "Play preview"}
          className={playing ? "text-brand" : "text-muted-foreground"}
          onClick={() => (playing ? stopPreview() : requestPreview(path))}
        >
          <HugeiconsIcon
            icon={playing ? StopIcon : PlayIcon}
            strokeWidth={2}
            className="fill-current"
          />
        </Button>
      </div>

      {info?.status === "ready" ? (
        <Waveform
          ref={waveform}
          role="img"
          aria-label={`Waveform of ${selection.name}`}
          peaks={info.info.peaks}
          color="var(--wf-display-foreground)"
          className="h-12 shrink-0 cursor-pointer bg-display"
          onClick={() => requestPreview(path)}
        />
      ) : info?.status === "error" ? (
        <p
          role="alert"
          title={info.message}
          className="flex h-12 shrink-0 items-center overflow-hidden rounded-sm border border-dashed border-destructive/50 px-2 text-destructive"
        >
          <span className="line-clamp-2">
            Could not read this sound. {info.message}
          </span>
        </p>
      ) : (
        <div
          role="status"
          aria-label="Reading the sound"
          className="h-12 shrink-0 animate-pulse rounded-sm bg-display/70"
        />
      )}

      {playError !== null ? (
        <p
          role="alert"
          title={playError}
          className="h-4 truncate text-destructive"
        >
          Could not play it. {playError}
        </p>
      ) : info?.status === "ready" ? (
        <Facts>
          <span>{formatDuration(info.info.durationSecs)}</span>
          <span>{formatSampleRate(info.info.sampleRate)}</span>
          <span>{formatChannels(info.info.channels)}</span>
        </Facts>
      ) : (
        <Facts>{null}</Facts>
      )}

      <div className="grid grid-cols-1 gap-1 @[232px]/browser:grid-cols-2">
        <Button
          size="sm"
          className="min-w-0"
          onClick={() => void addToRack(path)}
        >
          <span className="truncate">Add to rack</span>
        </Button>
        <ReplaceButton path={path} />
      </div>
    </>
  )
}

function ProjectPane({ selection }: { selection: Selection }) {
  return (
    <>
      <div className="flex h-6 items-center">
        <PaneName selection={selection} />
      </div>
      <p className="flex-1 text-muted-foreground">
        A Windfall project. Opening it replaces the project you have open.
      </p>
      <Button
        size="sm"
        variant="outline"
        className="self-start"
        onClick={() => void openProjectPath(selection.path)}
      >
        Open project
      </Button>
    </>
  )
}

function PaneHint() {
  return (
    <p className="m-auto max-w-56 text-center text-balance text-muted-foreground">
      Select a sound to hear it. Double-click it to add it to the channel rack,
      or drag it there.
    </p>
  )
}

/**
 * The strip under the tree for whatever is selected: a sound's waveform and
 * facts with the two ways to use it, or a project file. Its height does not
 * depend on the selection, so the tree does not jump while arrowing.
 */
export function PreviewPane() {
  const selection = useBrowserStore((state) => state.selected)

  return (
    <section
      aria-label="Preview"
      className="flex h-[164px] shrink-0 flex-col gap-1 border-t bg-chassis/60 p-1.5 @[232px]/browser:h-[136px]"
    >
      {selection?.kind === "audio" ? (
        <SoundPane key={selection.path} selection={selection} />
      ) : selection?.kind === "project" ? (
        <ProjectPane selection={selection} />
      ) : (
        <PaneHint />
      )}
    </section>
  )
}
