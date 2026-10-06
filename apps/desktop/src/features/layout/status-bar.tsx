import { runAction } from "@/lib/actions"
import { useEngineStore } from "@/lib/store/engine"
import { useHint, useHintStore } from "@/lib/store/hint"
import { useDirty, useProjectPath } from "@/lib/store/selectors"
import { fileName, formatSampleRate } from "@/lib/time"
import { cn } from "@/lib/utils"

function Dot({ className }: { className: string }) {
  return (
    <span
      aria-hidden
      className={cn("size-1.5 shrink-0 rounded-full", className)}
    />
  )
}

function EngineStatus() {
  const status = useEngineStore((state) => state.status)
  const hint = useHint("Audio output. Click to choose a device or buffer size")

  let content = (
    <>
      <Dot className="bg-foreground/30" />
      <span>Starting audio…</span>
    </>
  )
  if (status?.running) {
    content = (
      <>
        <Dot className="bg-ok" />
        <span className="truncate text-foreground/85">
          {status.host} {status.device}
        </span>
        <span className="shrink-0 font-readout">
          {formatSampleRate(status.sampleRate)}
        </span>
        <span className="shrink-0 font-readout">{status.bufferFrames} smp</span>
        <span className="shrink-0 font-readout">
          {status.latencyMs.toFixed(1)} ms
        </span>
      </>
    )
  } else if (status) {
    content = (
      <>
        <Dot className="bg-destructive" />
        <span className="truncate text-destructive">
          No audio. {status.error ?? "The output is not running."}
        </span>
      </>
    )
  }

  return (
    <button
      type="button"
      aria-label="Audio output status. Open audio settings"
      onClick={() => void runAction("options.settings")}
      className="flex h-full max-w-[45%] min-w-0 items-center gap-2 px-2 outline-none hover:bg-foreground/5 focus-visible:bg-foreground/10"
      {...hint}
    >
      {content}
    </button>
  )
}

function Hint() {
  const text = useHintStore((state) => state.text)
  return (
    <p aria-live="off" className="min-w-0 flex-1 truncate px-2">
      {text}
    </p>
  )
}

function SaveState() {
  const dirty = useDirty()
  const path = useProjectPath()

  let label = "Saved"
  if (path === null) label = "Not saved to a file yet"
  else if (dirty) label = "Edited"

  return (
    <div
      className="flex min-w-0 shrink items-center gap-2 px-2"
      title={path ?? undefined}
    >
      <Dot className={dirty ? "bg-brand" : "bg-foreground/30"} />
      <span className="shrink-0">{label}</span>
      {path !== null && (
        <span className="truncate text-foreground/85">{fileName(path)}</span>
      )}
    </div>
  )
}

/**
 * The bottom strip: the audio engine on the left, help for the control
 * under the pointer in the middle, and the state of the file on the right.
 */
export function StatusBar() {
  return (
    <footer className="flex h-6 shrink-0 items-center border-t bg-chassis text-[0.6875rem] text-muted-foreground">
      <EngineStatus />
      <Hint />
      <SaveState />
    </footer>
  )
}
