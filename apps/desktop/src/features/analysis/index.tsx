import { useEffect, useRef, useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { backend, errorMessage } from "@/lib/ipc"
import { receivePatch } from "@/lib/store/project"
import { getProjectGeneration, onProjectReplaced } from "@/lib/store/replaced"
import { usePlaylistStore } from "@/features/playlist/store"
import { parseManifest } from "./manifest"
import { retireJob } from "./retire"
import {
  validRange,
  type AnalysisCapability,
  type AnalysisJob,
  type AnalysisReview,
} from "./types"

export function AnalysisButton({
  clip,
  disabled = false,
}: {
  clip: number | null
  disabled?: boolean
}) {
  const [open, setOpen] = useState(false)
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger
        render={
          <Button
            size="sm"
            variant="outline"
            disabled={disabled || clip === null}
          />
        }
      >
        Analysis
      </DialogTrigger>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Native analysis</DialogTitle>
          <DialogDescription>
            Review outputs before applying one undo step. Original sources and
            applied output files remain available to history and saved projects.
          </DialogDescription>
        </DialogHeader>
        {open && clip !== null && <AnalysisPanel key={clip} clip={clip} />}
      </DialogContent>
    </Dialog>
  )
}

export function AnalysisPanel({ clip }: { clip: number }) {
  const [capability, setCapability] = useState<AnalysisCapability | null>(null)
  const [job, setJob] = useState<AnalysisJob | null>(null)
  const [review, setReview] = useState<AnalysisReview | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [stale, setStale] = useState(false)
  const [modelIndex, setModelIndex] = useState(0)
  const [start, setStart] = useState("0")
  const [end, setEnd] = useState("")
  const [manifest, setManifest] = useState("")
  const [localPath, setLocalPath] = useState("")
  const [replace, setReplace] = useState(false)
  const active = useRef(false)
  const retained = useRef<string | null>(null)
  const sequence = useRef(0)
  const aliveContext = useRef<() => boolean>(() => false)

  useEffect(() => {
    active.current = true
    const generation = getProjectGeneration()
    const selection = usePlaylistStore.getState().selection
    aliveContext.current = () =>
      active.current &&
      generation === getProjectGeneration() &&
      selection === usePlaylistStore.getState().selection
    const invalidate = () => {
      setStale(true)
      sequence.current += 1
    }
    const offProject = onProjectReplaced(invalidate)
    const offSelection = usePlaylistStore.subscribe((state) => {
      if (state.selection !== selection) invalidate()
    })
    void backend
      .analysisCapability()
      .then((value) => {
        if (aliveContext.current()) setCapability(value)
      })
      .catch((e: unknown) => {
        if (aliveContext.current()) setError(errorMessage(e))
      })
    return () => {
      active.current = false
      sequence.current += 1
      offProject()
      offSelection()
      if (retained.current)
        void retireJob(backend, retained.current).catch((e: unknown) =>
          console.error(errorMessage(e))
        )
    }
  }, [clip])

  useEffect(() => {
    if (
      !job ||
      !["queued", "running", "cancelling"].includes(job.status) ||
      stale
    )
      return
    let stopped = false
    const timer = setTimeout(() => {
      void backend
        .analysisStatus(job.job)
        .then((next) => {
          if (
            !stopped &&
            aliveContext.current() &&
            retained.current === next.job &&
            BigInt(next.sequence) >= BigInt(job.sequence)
          )
            setJob(next)
        })
        .catch((e: unknown) => {
          if (!stopped && aliveContext.current()) setError(errorMessage(e))
        })
    }, 200)
    return () => {
      stopped = true
      clearTimeout(timer)
    }
  }, [job, stale])

  async function action(work: () => Promise<void>) {
    if (busy || stale || !aliveContext.current()) return
    if (sequence.current >= Number.MAX_SAFE_INTEGER) {
      setError("Reopen this panel before continuing.")
      return
    }
    const request = ++sequence.current
    setBusy(true)
    setError(null)
    try {
      await work()
    } catch (e) {
      if (aliveContext.current() && request === sequence.current)
        setError(errorMessage(e))
    } finally {
      if (active.current && request === sequence.current) setBusy(false)
    }
  }
  const model = capability?.models[modelIndex]
  const terminal =
    job && ["cancelled", "failed", "consumed"].includes(job.status)
  return (
    <div className="flex flex-col gap-3 text-sm">
      {!capability && <p>Checking native availability…</p>}
      {capability && !capability.available && (
        <p role="status">
          {capability.reason ?? "No inference algorithm is available."}
        </p>
      )}
      {stale && (
        <p role="alert">
          The project or clip selection changed. Close and reopen Analysis.
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      <fieldset
        disabled={busy || stale || !capability?.native}
        className="flex flex-col gap-2"
      >
        <legend>Explicit local model import</legend>
        <p>
          Import a pinned manifest and local file. No download or algorithm
          installation occurs. Check its author, provenance and license before
          importing.
        </p>
        <Input
          aria-label="Local model path"
          value={localPath}
          onChange={(e) => setLocalPath(e.target.value)}
          maxLength={2048}
        />
        <Button
          variant="outline"
          onClick={() =>
            void action(async () => {
              const path = await backend.pickAnalysisModel()
              if (path && aliveContext.current()) setLocalPath(path)
            })
          }
        >
          Choose local model
        </Button>
        <textarea
          aria-label="Pinned model manifest JSON"
          maxLength={16384}
          value={manifest}
          onChange={(e) => setManifest(e.target.value)}
          className="min-h-24 rounded border p-2 font-mono text-xs"
        />
        <Button
          disabled={!manifest || !localPath}
          onClick={() =>
            void action(async () => {
              await backend.analysisModelImport(
                localPath,
                parseManifest(manifest)
              )
              const next = await backend.analysisCapability()
              if (aliveContext.current()) setCapability(next)
            })
          }
        >
          Verify and import local model
        </Button>
      </fieldset>
      <fieldset
        disabled={busy || stale || !capability?.available || job !== null}
        className="flex flex-col gap-2"
      >
        <legend>Submit rendered clip range</legend>
        <select
          aria-label="Pinned model"
          value={modelIndex}
          onChange={(e) => setModelIndex(Number(e.target.value))}
        >
          {capability?.models.map((m, index) => (
            <option key={`${m.id}/${m.version}/${m.revision}`} value={index}>
              {m.id} {m.version} · revision {m.revision}
            </option>
          ))}
        </select>
        <p>
          Frames refer to the rendered clip before mixer effects. A partial
          range adds a derived clip at the nearest timeline tick.
        </p>
        <Input
          aria-label="Start frame"
          inputMode="numeric"
          value={start}
          onChange={(e) => setStart(e.target.value)}
          maxLength={20}
        />
        <Input
          aria-label="End frame exclusive"
          inputMode="numeric"
          value={end}
          onChange={(e) => setEnd(e.target.value)}
          maxLength={20}
        />
        <Button
          disabled={!model || !validRange(start, end)}
          onClick={() =>
            void action(async () => {
              if (!model) return
              const next = await backend.analysisSubmit({
                clip,
                modelId: model.id,
                modelVersion: model.version,
                modelRevision: model.revision,
                startFrame: start,
                endFrame: end,
              })
              if (!aliveContext.current()) {
                await retireJob(backend, next.job)
                return
              }
              retained.current = next.job
              setJob(next)
            })
          }
        >
          Submit analysis
        </Button>
      </fieldset>
      {job && (
        <div className="flex flex-col gap-2">
          <p role="status">
            Job {job.job}: {job.status} · work {job.completedWork}/
            {job.maximumWork}
          </p>
          {job.failure && <p role="alert">{job.failure}</p>}
          <Button
            disabled={busy || !!terminal}
            onClick={() =>
              void action(async () => {
                const next = await backend.analysisCancel(job.job)
                if (aliveContext.current()) {
                  setReview(null)
                  setJob(next)
                }
              })
            }
          >
            Cancel job
          </Button>
          <Button
            disabled={busy || job.status !== "ready" || stale}
            onClick={() =>
              void action(async () => {
                const next = await backend.analysisReview(job.ticket)
                if (aliveContext.current() && retained.current === next.job.job)
                  setReview(next)
              })
            }
          >
            Review outputs
          </Button>
          <Button
            disabled={busy}
            onClick={() =>
              void action(async () => {
                await backend.analysisRetryCleanup(job.job)
                const next = await backend.analysisStatus(job.job)
                if (aliveContext.current()) setJob(next)
              })
            }
          >
            Retry owned cleanup
          </Button>
          <Button
            disabled={busy || !terminal}
            onClick={() =>
              void action(async () => {
                await backend.analysisForget(job.job)
                retained.current = null
                if (aliveContext.current()) {
                  setJob(null)
                  setReview(null)
                }
              })
            }
          >
            Forget retired job
          </Button>
        </div>
      )}
      {review && (
        <div className="flex flex-col gap-2 break-all">
          <p>Source SHA256: {review.sourceSha256}</p>
          <p>
            Model: {review.model.id} {review.model.version}, SHA256{" "}
            {review.model.sha256}
          </p>
          <p>
            {review.model.provenance.author} · {review.model.provenance.origin}{" "}
            · {review.model.provenance.sourceRevision}
          </p>
          <p>
            License: {review.model.provenance.licenseSpdx} ·{" "}
            {review.model.provenance.licenseReference}
          </p>
          {review.artifacts.map((output) => (
            <p key={output.name}>
              {output.role}: {output.frames} frames, {output.channels} channels
              at {output.sampleRate} Hz; origin {output.frameOrigin}; SHA256{" "}
              {output.sha256}
            </p>
          ))}
          <label>
            <input
              type="checkbox"
              checked={replace}
              disabled={
                review.startFrame !== "0" ||
                review.endFrame !== review.inputFrames
              }
              onChange={(e) => setReplace(e.target.checked)}
            />{" "}
            Replace original clip (complete range only)
          </label>
          <Button
            disabled={busy || stale || job?.status !== "ready"}
            onClick={() =>
              void action(async () => {
                const result = await backend.analysisApply({
                  ticket: review.job.ticket,
                  request: review.job.request,
                  replaceOriginal: replace,
                })
                if (!aliveContext.current()) return
                receivePatch(result.patch)
                setReview(null)
                setJob(await backend.analysisStatus(review.job.job))
              })
            }
          >
            Apply reviewed outputs
          </Button>
        </div>
      )}
      {busy && (
        <Button
          variant="outline"
          onClick={() =>
            void backend
              .analysisCancelPreparation()
              .catch((e: unknown) => setError(errorMessage(e)))
          }
        >
          Cancel active analysis preparation
        </Button>
      )}
    </div>
  )
}
