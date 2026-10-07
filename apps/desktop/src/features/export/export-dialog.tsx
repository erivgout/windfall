import { useEffect, useRef, useState, type FormEvent } from "react"
import { toast } from "sonner"

import type { ExportOptions, PlayMode } from "@/bindings"
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
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from "@/components/ui/progress"
import { reportError } from "@/lib/errors"
import { projectName } from "@/lib/flows/project"
import { backend, errorMessage } from "@/lib/ipc"
import { useEngineStore } from "@/lib/store/engine"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"

import { Choice, type Option } from "./choice"
import { changeExportFormat, FormatControls } from "./format-controls"
import { StemControls } from "./stem-controls"

import {
  cleanExportPath,
  exportedName,
  MAX_PATTERN_LOOPS,
  MAX_TAIL_SECS,
  parsePatternLoops,
  parseTailSecs,
  replaceExportExtension,
} from "./names"

const MODES: Option<PlayMode>[] = [
  { value: "pattern", label: "The selected pattern" },
  { value: "song", label: "The whole song" },
]

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
  const [cancelling, setCancelling] = useState(false)
  // An empty path is only called out once Export was pressed without one.
  const [pathAsked, setPathAsked] = useState(false)
  const exporting = run !== null && run.error === null
  // The path the running export was started with, which is the name its
  // progress arrives under.
  const sent = useRef<ExportOptions | null>(null)

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
  const stemsError =
    draft.stems?.tracks?.length === 0 && !draft.stems.includeMix
      ? "Choose at least one track or include the full mix."
      : null

  useEffect(
    () =>
      backend.onExportProgress((progress) => {
        const options = sent.current
        if (progress.path !== options?.path) return
        if (progress.error !== null) {
          setRun({ fraction: progress.fraction, error: progress.error })
        } else if (progress.done) {
          sent.current = null
          setCancelling(false)
          if (progress.cancelled) {
            setRun(null)
            toast("Export cancelled")
            return
          }
          toast.success("Exported", {
            description: options.stems
              ? progress.files
                ? `${progress.files.length} files`
                : "Mixer stems"
              : exportedName(progress.path, options.format),
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
      const picked = await backend.pickExportPath(projectName(), draft.format)
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
    if (stemsError) return focus("export-stem-selection")

    const options: ExportOptions = { ...draft, path, patternLoops, tailSecs }
    // The field shows the name the file gets.
    set("path", path)
    sent.current = options
    setCancelling(false)
    setRun({ fraction: 0, error: null })
    try {
      await backend.exportAudio(options)
    } catch (error) {
      setRun({ fraction: 0, error: errorMessage(error) })
    }
  }

  async function cancelExport() {
    setCancelling(true)
    try {
      await backend.exportCancel()
    } catch (error) {
      setCancelling(false)
      reportError(error, "Could not cancel the export")
    }
  }

  return (
    <form noValidate onSubmit={onSubmit}>
      <FieldGroup>
        <FormatControls
          settings={draft}
          disabled={exporting}
          set={set}
          onFormat={(format) =>
            setDraft((current) => ({
              ...changeExportFormat(current, format),
              path: replaceExportExtension(current.path, format),
            }))
          }
        />
        <Field data-invalid={pathError !== null || undefined}>
          <FieldLabel htmlFor="export-path">Save to</FieldLabel>
          <div className="flex gap-2">
            <Input
              id="export-path"
              value={draft.path}
              placeholder={`Choose where to save the ${draft.format.toUpperCase()} file`}
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

        <FieldGroup className="grid grid-cols-2 gap-3">
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
        </FieldGroup>

        <StemControls
          value={draft.stems}
          disabled={exporting}
          onChange={(stems) => set("stems", stems)}
        />
        {stemsError && <FieldError>{stemsError}</FieldError>}

        {exporting && (
          <Progress value={run.fraction * 100}>
            <ProgressLabel>Exporting</ProgressLabel>
            <ProgressValue />
          </Progress>
        )}
        {run?.error && <FieldError>The export failed. {run.error}</FieldError>}

        <DialogFooter>
          {exporting && (
            <Button
              type="button"
              variant="outline"
              disabled={cancelling}
              onClick={() => void cancelExport()}
            >
              {cancelling ? "Cancelling…" : "Cancel export"}
            </Button>
          )}
          <Button type="button" variant="outline" onClick={onDone}>
            {exporting ? "Close" : "Cancel"}
          </Button>
          <Button type="submit" disabled={exporting}>
            Export
          </Button>
        </DialogFooter>
      </FieldGroup>
    </form>
  )
}

/** Exports the pattern or song to audio files and shows progress. */
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
      <DialogContent className="max-h-[calc(100dvh-4rem)] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Export audio</DialogTitle>
          <DialogDescription>
            Save the selected pattern or song as a mix or separate mixer tracks.
          </DialogDescription>
        </DialogHeader>
        <ExportForm onDone={closeDialog} />
      </DialogContent>
    </Dialog>
  )
}
