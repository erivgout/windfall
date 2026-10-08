import { useEffect, useLayoutEffect, useRef, useState } from "react"
import { ActionButton } from "@/components/action-button"
import { Input } from "@/components/ui/input"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { backend, errorMessage } from "@/lib/ipc"
import {
  onHistoryNavigation,
  receivePatch,
  useProjectStore,
} from "@/lib/store/project"
import { onProjectReplaced } from "@/lib/store/replaced"
import { usePlaylistStore } from "@/features/playlist/store"
import { parseManifest } from "./manifest"
import {
  recoveryFailure,
  removeRecoveryJob,
  reserveRecovery,
  retireTrackedJob,
  updateRecoveryJob,
  useAnalysisRecovery,
} from "./recovery"
import {
  bindAnalysisPanel,
  captureAnalysisClip,
  closeAnalysis,
  useAnalysisDialog,
  type AnalysisCommand,
} from "./actions"
import {
  type AnalysisCapability,
  type AnalysisJob,
  type AnalysisReview,
} from "./types"

export function AnalysisButton() {
  const target = useAnalysisDialog((state) => state.target)
  return (
    <Dialog
      open={target !== null}
      onOpenChange={(open) => {
        if (!open) closeAnalysis()
      }}
    >
      <ActionButton action="analysis.open" size="sm" variant="outline" />
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Native analysis</DialogTitle>
          <DialogDescription>
            Review outputs before applying one undo step. Original sources and
            applied output files remain available to history and saved projects.
          </DialogDescription>
        </DialogHeader>
        {target && (
          <AnalysisPanel
            key={`${target.generation}/${target.id}`}
            clip={target.id}
            capture={target}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

export function AnalysisPanel({
  clip,
  capture,
}: {
  clip: number
  capture?: ReturnType<typeof captureAnalysisClip>
}) {
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
  const working = useRef(false)

  useEffect(() => {
    active.current = true
    const captured = capture ?? captureAnalysisClip(clip)
    aliveContext.current = () =>
      active.current && captured !== null && captured.current()
    const invalidate = () => {
      setStale(true)
      sequence.current += 1
    }
    let retired = false
    let offProject = () => {}
    let offSelection = () => {}
    let offSource = () => {}
    let offHistory = () => {}
    const retireLifetime = () => {
      if (retired) return
      retired = true
      active.current = false
      aliveContext.current = () => false
      sequence.current += 1
      offProject()
      offSelection()
      offSource()
      offHistory()
      const id = retained.current
      retained.current = null
      if (id) void retireTrackedJob(id)
    }
    offProject = onProjectReplaced(() => {
      invalidate()
      retireLifetime()
    })
    const recheck = () => {
      if (!captured?.current()) invalidate()
    }
    offSelection = usePlaylistStore.subscribe(recheck)
    offSource = useProjectStore.subscribe(recheck)
    offHistory = onHistoryNavigation(invalidate)
    if (!captured) invalidate()
    void backend
      .analysisCapability()
      .then((value) => {
        if (aliveContext.current()) setCapability(value)
      })
      .catch((e: unknown) => {
        if (aliveContext.current()) setError(errorMessage(e))
      })
    return retireLifetime
  }, [clip, capture])

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
  useEffect(() => {
    if (job) updateRecoveryJob(job)
  }, [job])

  async function action(work: () => Promise<void>, retirement = false) {
    if (
      working.current ||
      !active.current ||
      (!retirement && (stale || !aliveContext.current()))
    )
      return
    if (sequence.current >= Number.MAX_SAFE_INTEGER) {
      setError("Reopen this panel before continuing.")
      return
    }
    const request = ++sequence.current
    working.current = true
    setBusy(true)
    setError(null)
    try {
      await work()
    } catch (e) {
      const message =
        retirement && job ? recoveryFailure(job.job, e) : errorMessage(e)
      if (
        active.current &&
        (retirement || aliveContext.current()) &&
        request === sequence.current
      )
        setError(message)
    } finally {
      working.current = false
      if (active.current) setBusy(false)
    }
  }
  const model = capability?.models[modelIndex]
  useLayoutEffect(() => {
    if (!active.current) return
    return bindAnalysisPanel({
      clip,
      current: () => aliveContext.current(),
      stale,
      busy,
      capability,
      job,
      review,
      modelIndex,
      start,
      end,
      manifest,
      localPath,
      replace,
      execute: async (command: AnalysisCommand) => {
        if (command === "cancelPreparation") {
          try {
            await backend.analysisCancelPreparation()
          } catch (e) {
            if (active.current) setError(errorMessage(e))
          }
          return
        }
        await action(
          async () => {
            switch (command) {
              case "chooseModel": {
                const path = await backend.pickAnalysisModel()
                if (path && aliveContext.current()) setLocalPath(path)
                break
              }
              case "importModel": {
                await backend.analysisModelImport(
                  localPath,
                  parseManifest(manifest)
                )
                const next = await backend.analysisCapability()
                if (aliveContext.current()) setCapability(next)
                break
              }
              case "submit": {
                if (!model) return
                const finish = reserveRecovery()
                let next: AnalysisJob
                try {
                  next = await backend.analysisSubmit({
                    clip,
                    modelId: model.id,
                    modelVersion: model.version,
                    modelRevision: model.revision,
                    startFrame: start,
                    endFrame: end,
                  })
                  finish(next)
                } catch (error) {
                  finish(null)
                  throw error
                }
                if (!aliveContext.current()) {
                  await retireTrackedJob(next.job)
                  return
                }
                retained.current = next.job
                setJob(next)
                break
              }
              case "cancel": {
                if (!job) return
                const next = await backend.analysisCancel(job.job)
                if (active.current && retained.current === next.job) {
                  setReview(null)
                  setJob(next)
                }
                break
              }
              case "review": {
                if (!job) return
                const next = await backend.analysisReview(job.ticket)
                if (aliveContext.current() && retained.current === next.job.job)
                  setReview(next)
                break
              }
              case "retryCleanup": {
                if (!job) return
                await backend.analysisRetryCleanup(job.job)
                const next = await backend.analysisStatus(job.job)
                if (active.current && retained.current === next.job)
                  setJob(next)
                break
              }
              case "forget": {
                if (!job) return
                await backend.analysisForget(job.job)
                removeRecoveryJob(job.job)
                retained.current = null
                if (active.current) {
                  setJob(null)
                  setReview(null)
                }
                break
              }
              case "apply": {
                if (!review) return
                const result = await backend.analysisApply({
                  ticket: review.job.ticket,
                  request: review.job.request,
                  replaceOriginal: replace,
                })
                if (!aliveContext.current()) return
                receivePatch(result.patch)
                setReview(null)
                const next = await backend.analysisStatus(review.job.job)
                if (active.current && retained.current === next.job)
                  setJob(next)
                break
              }
            }
          },
          command === "cancel" ||
            command === "retryCleanup" ||
            command === "forget"
        )
      },
    })
  })
  return (
    <div className="flex flex-col gap-3 text-sm">
      <RecoveryControls />
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
        <ActionButton action="analysis.chooseModel" variant="outline" />
        <textarea
          aria-label="Pinned model manifest JSON"
          maxLength={16384}
          value={manifest}
          onChange={(e) => setManifest(e.target.value)}
          className="min-h-24 rounded border p-2 font-mono text-xs"
        />
        <ActionButton action="analysis.importModel" />
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
        <ActionButton action="analysis.submit" />
      </fieldset>
      {job && (
        <div className="flex flex-col gap-2">
          <p role="status">
            Job {job.job}: {job.status} · work {job.completedWork}/
            {job.maximumWork}
          </p>
          {job.failure && <p role="alert">{job.failure}</p>}
          <ActionButton action="analysis.cancel" />
          <ActionButton action="analysis.review" />
          <ActionButton action="analysis.retryCleanup" />
          <ActionButton action="analysis.forget" />
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
          <ActionButton action="analysis.apply" />
        </div>
      )}
      {busy && (
        <ActionButton action="analysis.cancelPreparation" variant="outline" />
      )}
    </div>
  )
}

function RecoveryControls() {
  const jobs = useAnalysisRecovery((state) => state.jobs)
  const selected = useAnalysisRecovery((state) => state.selected)
  const dismissed = jobs.filter((item) => item.dismissed)
  if (dismissed.length === 0) return null
  const current = dismissed.find((item) => item.job.job === selected)
  return (
    <div className="flex flex-col gap-2">
      <p>
        Retained analysis jobs require explicit cleanup. These controls cannot
        apply outputs to the current clip.
      </p>
      <select
        aria-label="Retained analysis job"
        value={selected ?? ""}
        onChange={(event) =>
          useAnalysisRecovery.setState({ selected: event.target.value })
        }
      >
        {dismissed.map((item) => (
          <option key={item.job.job} value={item.job.job}>
            {item.job.job} · {item.job.status}
          </option>
        ))}
      </select>
      {current && (
        <p>
          Retained job {current.job.job}: {current.job.status}
        </p>
      )}
      {current?.error && <p role="alert">{current.error}</p>}
      <ActionButton action="analysis.recoveryCancel" />
      <ActionButton action="analysis.recoveryRetry" />
      <ActionButton action="analysis.recoveryForget" />
    </div>
  )
}
