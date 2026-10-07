import { useEffect, useRef, useState, type FormEvent } from "react"
import { toast } from "sonner"

import type { BitDepth, ExportOptions, PlayMode } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Field, FieldError, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from "@/components/ui/progress"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { reportError } from "@/lib/errors"
import { projectName } from "@/lib/flows/project"
import { backend, errorMessage } from "@/lib/ipc"
import { useEngineStore } from "@/lib/store/engine"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { formatSampleRate } from "@/lib/time"

import {
  cleanExportPath,
  exportedName,
  MAX_PATTERN_LOOPS,
  MAX_TAIL_SECS,
  parsePatternLoops,
  parseTailSecs,
} from "./names"

type Option<T> = { value: T; label: string }

const MODES: Option<PlayMode>[] = [
  { value: "pattern", label: "The selected pattern" },
  { value: "song", label: "The whole song" },
]

const BIT_DEPTHS: Option<BitDepth>[] = [
  { value: "int16", label: "16 bit" },
  { value: "int24", label: "24 bit" },
  { value: "float32", label: "32 bit float" },
]

const SAMPLE_RATES: Option<number>[] = [44_100, 48_000, 88_200, 96_000].map(
  (rate) => ({ value: rate, label: formatSampleRate(rate) })
)

function Choice<T extends string | number>({
  id,
  value,
  options,
  disabled,
  onChange,
}: {
  id: string
  value: T
  options: Option<T>[]
  disabled: boolean
  onChange(value: T): void
}) {
  return (
    <Select
      items={options}
      value={value}
      disabled={disabled}
      onValueChange={(next: T | null) => {
        if (next !== null) onChange(next)
      }}
    >
      <SelectTrigger id={id} className="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent alignItemWithTrigger={false}>
        <SelectGroup>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}

type Run = { fraction: number; error: string | null }

/** What the form holds. The two numbers stay text until they are sent. */
type Draft = Omit<ExportOptions, "patternLoops" | "tailSecs"> & {
  patternLoops: string
  tailSecs: string
}

const NO_PATH = "Choose where to save the file."
const BAD_LOOPS = `Enter a whole number from 1 to ${MAX_PATTERN_LOOPS}.`
const BAD_TAIL = `Enter a number of seconds from 0 to ${MAX_TAIL_SECS}.`

function ExportForm({ onDone }: { onDone(): void }) {
  const [draft, setDraft] = useState<Draft>(() => ({
    path: "",
    format: "wav",
    bitDepth: "int24",
    sampleRate: useEngineStore.getState().status?.sampleRate ?? 44_100,
    mode: useTransportStore.getState().mode,
    patternLoops: "4",
    tailSecs: "10",
    autoTail: true,
  }))
  const [run, setRun] = useState<Run | null>(null)
  // An empty path is only called out once Export was pressed without one.
  const [pathAsked, setPathAsked] = useState(false)
  const exporting = run !== null && run.error === null
  // The path the running export was started with, which is the name its
  // progress arrives under.
  const sentPath = useRef<string | null>(null)

  const set = <K extends keyof Draft>(key: K, value: Draft[K]) =>
    setDraft((current) => ({ ...current, [key]: value }))

  const path = cleanExportPath(draft.path)
  const tailSecs = parseTailSecs(draft.tailSecs)
  // Song mode does not use the count, so a bad one must not be in its way.
  const patternLoops =
    draft.mode === "song"
      ? (parsePatternLoops(draft.patternLoops) ?? 1)
      : parsePatternLoops(draft.patternLoops)
  const pathError = pathAsked && path === "" ? NO_PATH : null
  const loopsError = patternLoops === null ? BAD_LOOPS : null
  const tailError = tailSecs === null ? BAD_TAIL : null

  useEffect(
    () =>
      backend.onExportProgress((progress) => {
        if (progress.path !== sentPath.current) return
        if (progress.error !== null) {
          setRun({ fraction: progress.fraction, error: progress.error })
        } else if (progress.done) {
          toast.success("Exported", {
            description: exportedName(progress.path),
          })
          onDone()
        } else {
          setRun({ fraction: progress.fraction, error: null })
        }
      }),
    [onDone]
  )

  async function choosePath() {
    try {
      const picked = await backend.pickExportPath(projectName())
      if (picked !== null) set("path", picked)
    } catch (error) {
      reportError(error, "Could not choose a file")
    }
  }

  // The form checks itself: the browser's own validation would turn a
  // press on Export into nothing at all when a number is out of range.
  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = event.currentTarget
    const focus = (id: string) =>
      form.querySelector<HTMLElement>(`#${id}`)?.focus()
    if (path === "") {
      setPathAsked(true)
      focus("export-path")
      return
    }
    if (patternLoops === null) return focus("export-loops")
    if (tailSecs === null) return focus("export-tail")

    const options: ExportOptions = { ...draft, path, patternLoops, tailSecs }
    // The field shows the name the file gets.
    set("path", path)
    sentPath.current = path
    setRun({ fraction: 0, error: null })
    try {
      await backend.exportAudio(options)
    } catch (error) {
      setRun({ fraction: 0, error: errorMessage(error) })
    }
  }

  return (
    <form noValidate onSubmit={onSubmit} className="grid gap-4">
      <Field data-invalid={pathError !== null || undefined}>
        <FieldLabel htmlFor="export-path">Save to</FieldLabel>
        <div className="flex gap-2">
          <Input
            id="export-path"
            value={draft.path}
            placeholder="Choose where to save the WAV file"
            disabled={exporting}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={pathError !== null || undefined}
            aria-describedby={pathError ? "export-path-error" : undefined}
            onChange={(event) => set("path", event.target.value)}
          />
          <Button
            type="button"
            variant="outline"
            disabled={exporting}
            onClick={() => void choosePath()}
          >
            Choose…
          </Button>
        </div>
        {pathError && (
          <FieldError id="export-path-error">{pathError}</FieldError>
        )}
      </Field>

      <div className="grid grid-cols-2 gap-3">
        <Field>
          <FieldLabel htmlFor="export-mode">Render</FieldLabel>
          <Choice
            id="export-mode"
            value={draft.mode}
            options={MODES}
            disabled={exporting}
            onChange={(mode) => set("mode", mode)}
          />
        </Field>
        <Field data-invalid={loopsError !== null || undefined}>
          <FieldLabel htmlFor="export-loops">
            Times through the pattern
          </FieldLabel>
          <Input
            id="export-loops"
            type="number"
            min={1}
            max={MAX_PATTERN_LOOPS}
            value={draft.patternLoops}
            disabled={exporting || draft.mode === "song"}
            aria-invalid={loopsError !== null || undefined}
            aria-describedby={loopsError ? "export-loops-error" : undefined}
            onChange={(event) => set("patternLoops", event.target.value)}
          />
          {loopsError && (
            <FieldError id="export-loops-error">{loopsError}</FieldError>
          )}
        </Field>
        <Field>
          <FieldLabel htmlFor="export-depth">Bit depth</FieldLabel>
          <Choice
            id="export-depth"
            value={draft.bitDepth}
            options={BIT_DEPTHS}
            disabled={exporting}
            onChange={(bitDepth) => set("bitDepth", bitDepth)}
          />
        </Field>
        <Field>
          <FieldLabel htmlFor="export-rate">Sample rate</FieldLabel>
          <Choice
            id="export-rate"
            value={draft.sampleRate}
            options={SAMPLE_RATES}
            disabled={exporting}
            onChange={(sampleRate) => set("sampleRate", sampleRate)}
          />
        </Field>
        <Field orientation="horizontal" className="col-span-2">
          <Checkbox
            id="export-auto-tail"
            checked={draft.autoTail}
            disabled={exporting}
            onCheckedChange={(autoTail) => set("autoTail", autoTail)}
          />
          <FieldLabel htmlFor="export-auto-tail">
            Stop when the tail has faded
          </FieldLabel>
        </Field>
        <Field
          className="col-span-2"
          data-invalid={tailError !== null || undefined}
        >
          <FieldLabel htmlFor="export-tail">
            {draft.autoTail
              ? "Keep at most this many seconds after the end"
              : "Seconds to keep after the end, for tails to ring out"}
          </FieldLabel>
          <Input
            id="export-tail"
            type="number"
            min={0}
            max={MAX_TAIL_SECS}
            // What the arrows move by. Any number in the range is taken.
            step={0.5}
            value={draft.tailSecs}
            disabled={exporting}
            aria-invalid={tailError !== null || undefined}
            aria-describedby={tailError ? "export-tail-error" : undefined}
            onChange={(event) => set("tailSecs", event.target.value)}
          />
          {tailError && (
            <FieldError id="export-tail-error">{tailError}</FieldError>
          )}
        </Field>
      </div>

      {exporting && (
        <Progress value={run.fraction * 100}>
          <ProgressLabel>Exporting</ProgressLabel>
          <ProgressValue />
        </Progress>
      )}
      {run?.error && (
        <p
          role="alert"
          className="rounded-md bg-destructive/10 px-2.5 py-1.5 text-destructive"
        >
          The export failed. {run.error}
        </p>
      )}

      <DialogFooter>
        <Button type="button" variant="outline" onClick={onDone}>
          {exporting ? "Close" : "Cancel"}
        </Button>
        <Button type="submit" disabled={exporting}>
          Export
        </Button>
      </DialogFooter>
    </form>
  )
}

/** Renders the pattern or the song to a WAV file and shows the progress. */
export function ExportDialog() {
  const open = useUiStore((state) => state.dialog === "export")
  const closeDialog = useUiStore((state) => state.closeDialog)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeDialog()
      }}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Export audio</DialogTitle>
          <DialogDescription>
            The file sounds the same as playback: it is made by the same engine.
          </DialogDescription>
        </DialogHeader>
        <ExportForm onDone={closeDialog} />
      </DialogContent>
    </Dialog>
  )
}
