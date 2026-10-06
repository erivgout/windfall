import { useEffect, useState, type FormEvent } from "react"
import { toast } from "sonner"

import type { BitDepth, ExportOptions, PlayMode } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Field, FieldLabel } from "@/components/ui/field"
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
import { backend, errorMessage } from "@/lib/ipc"
import { useEngineStore } from "@/lib/store/engine"
import { useProjectStore } from "@/lib/store/project"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { fileName, formatSampleRate } from "@/lib/time"

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

function ExportForm({ onDone }: { onDone(): void }) {
  const [options, setOptions] = useState<ExportOptions>(() => ({
    path: "",
    format: "wav",
    bitDepth: "int24",
    sampleRate: useEngineStore.getState().status?.sampleRate ?? 44_100,
    mode: useTransportStore.getState().mode,
    patternLoops: 4,
    tailSecs: 1,
  }))
  const [run, setRun] = useState<Run | null>(null)
  const exporting = run !== null && run.error === null

  const set = <K extends keyof ExportOptions>(
    key: K,
    value: ExportOptions[K]
  ) => setOptions((current) => ({ ...current, [key]: value }))

  useEffect(
    () =>
      backend.onExportProgress((progress) => {
        if (progress.path !== options.path) return
        if (progress.error !== null) {
          setRun({ fraction: progress.fraction, error: progress.error })
        } else if (progress.done) {
          toast.success("Exported", { description: fileName(progress.path) })
          onDone()
        } else {
          setRun({ fraction: progress.fraction, error: null })
        }
      }),
    [options.path, onDone]
  )

  async function choosePath() {
    try {
      const name =
        useProjectStore.getState().project.settings.name || "Untitled"
      const path = await backend.pickExportPath(name)
      if (path !== null) set("path", path)
    } catch (error) {
      reportError(error, "Could not choose a file")
    }
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setRun({ fraction: 0, error: null })
    try {
      await backend.exportAudio(options)
    } catch (error) {
      setRun({ fraction: 0, error: errorMessage(error) })
    }
  }

  return (
    <form onSubmit={onSubmit} className="grid gap-4">
      <Field>
        <FieldLabel htmlFor="export-path">Save to</FieldLabel>
        <div className="flex gap-2">
          <Input
            id="export-path"
            value={options.path}
            placeholder="Choose where to save the WAV file"
            disabled={exporting}
            autoComplete="off"
            spellCheck={false}
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
      </Field>

      <div className="grid grid-cols-2 gap-3">
        <Field>
          <FieldLabel htmlFor="export-mode">Render</FieldLabel>
          <Choice
            id="export-mode"
            value={options.mode}
            options={MODES}
            disabled={exporting}
            onChange={(mode) => set("mode", mode)}
          />
        </Field>
        <Field>
          <FieldLabel htmlFor="export-loops">
            Times through the pattern
          </FieldLabel>
          <Input
            id="export-loops"
            type="number"
            min={1}
            max={64}
            value={options.patternLoops}
            disabled={exporting || options.mode === "song"}
            onChange={(event) =>
              set(
                "patternLoops",
                Math.max(1, Math.round(event.target.valueAsNumber || 1))
              )
            }
          />
        </Field>
        <Field>
          <FieldLabel htmlFor="export-depth">Bit depth</FieldLabel>
          <Choice
            id="export-depth"
            value={options.bitDepth}
            options={BIT_DEPTHS}
            disabled={exporting}
            onChange={(bitDepth) => set("bitDepth", bitDepth)}
          />
        </Field>
        <Field>
          <FieldLabel htmlFor="export-rate">Sample rate</FieldLabel>
          <Choice
            id="export-rate"
            value={options.sampleRate}
            options={SAMPLE_RATES}
            disabled={exporting}
            onChange={(sampleRate) => set("sampleRate", sampleRate)}
          />
        </Field>
        <Field className="col-span-2">
          <FieldLabel htmlFor="export-tail">
            Seconds to keep after the end, for tails to ring out
          </FieldLabel>
          <Input
            id="export-tail"
            type="number"
            min={0}
            max={30}
            step={0.5}
            value={options.tailSecs}
            disabled={exporting}
            onChange={(event) =>
              set("tailSecs", Math.max(0, event.target.valueAsNumber || 0))
            }
          />
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
        <Button
          type="submit"
          disabled={exporting || options.path.trim() === ""}
        >
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
