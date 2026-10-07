import { useEffect, useId, useRef, useState } from "react"
import type { Clip } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { backend, errorMessage } from "@/lib/ipc"
import { receivePatch, useProjectStore } from "@/lib/store/project"
import { PPQ } from "@/lib/units"
import { usePlaylistStore } from "@/features/playlist/store"
import type { SliceOptions, SliceReview } from "./types"

/** Entry point for one selected playlist clip; markers remain review-only. */
export function SliceControls({ clips }: { clips: Clip[] }) {
  const [open, setOpen] = useState(false)
  const single = clips.length === 1 && clips[0].content.type === "audio"
  return (
    <Dialog open={open && single} onOpenChange={setOpen}>
      <DialogTrigger
        disabled={!single}
        render={<Button size="sm" variant="outline" />}
      >
        Slice clip
      </DialogTrigger>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Slice audio clip</DialogTitle>
          <DialogDescription>
            Review the cut markers, then split into clips linked to the original
            file. Apply is one undo step.
          </DialogDescription>
        </DialogHeader>
        {open && single && (
          <SliceForm
            key={clips[0].id}
            clip={clips[0]}
            onApplied={() => setOpen(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

function SliceForm({ clip, onApplied }: { clip: Clip; onApplied: () => void }) {
  const id = useId()
  const [mode, setMode] = useState<"grid" | "transients">("grid")
  const [grid, setGrid] = useState(PPQ)
  const [sensitivity, setSensitivity] = useState("50")
  const [review, setReview] = useState<SliceReview | null>(null)
  const [reviewRevision, setReviewRevision] = useState(-1)
  const [selected, setSelected] = useState<number[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState("")
  const revision = useProjectStore((s) => s.revision)
  const alive = useRef(false)
  const retained = useRef<SliceReview | null>(null)
  useEffect(() => {
    alive.current = true
    return () => {
      alive.current = false
      if (retained.current)
        void backend.sliceDiscard(retained.current.token).catch(() => {})
    }
  }, [])
  const stale = review !== null && reviewRevision !== revision
  const invalidSensitivity =
    sensitivity.trim() === "" ||
    !Number.isFinite(Number(sensitivity)) ||
    Number(sensitivity) < 0 ||
    Number(sensitivity) > 100
  function reset() {
    if (retained.current)
      void backend.sliceDiscard(retained.current.token).catch(() => {})
    retained.current = null
    setReview(null)
    setSelected([])
    setError("")
  }
  async function analyze() {
    reset()
    const atRevision = useProjectStore.getState().revision
    const options: SliceOptions =
      mode === "grid"
        ? { mode, gridTicks: grid }
        : { mode, sensitivity: Number(sensitivity) / 100 }
    setBusy(true)
    try {
      const result = await backend.sliceAnalyze(clip.id, options)
      if (
        !alive.current ||
        atRevision !== useProjectStore.getState().revision
      ) {
        void backend.sliceDiscard(result.token).catch(() => {})
        if (alive.current)
          setError("The project changed. Analyze the clip again.")
        return
      }
      retained.current = result
      setReview(result)
      setReviewRevision(atRevision)
      setSelected(result.analysis.markers.map((m) => m.tick))
    } catch (failure) {
      if (alive.current) setError(errorMessage(failure))
    } finally {
      if (alive.current) setBusy(false)
    }
  }
  async function apply() {
    if (!review || stale) return
    setBusy(true)
    setError("")
    try {
      const result = await backend.sliceApply(review.token, selected)
      receivePatch(result.patch)
      usePlaylistStore.getState().select(result.created)
      retained.current = null
      if (alive.current) onApplied()
    } catch (failure) {
      if (alive.current) setError(errorMessage(failure))
    } finally {
      if (alive.current) setBusy(false)
    }
  }
  function toggle(tick: number) {
    setSelected((current) =>
      current.includes(tick)
        ? current.filter((t) => t !== tick)
        : [...current, tick].sort((a, b) => a - b)
    )
  }
  return (
    <FieldGroup>
      {backend.kind === "mock" && (
        <FieldDescription>
          Browser demonstration: audio and waveforms are simulated. Native
          analysis uses the loaded file.
        </FieldDescription>
      )}
      <Field>
        <FieldLabel>Find markers</FieldLabel>
        <ToggleGroup
          value={[mode]}
          disabled={busy}
          onValueChange={(value) => {
            if (value[0] === "grid" || value[0] === "transients") {
              reset()
              setMode(value[0])
            }
          }}
        >
          <ToggleGroupItem value="grid">Beat grid</ToggleGroupItem>
          <ToggleGroupItem value="transients">Transients</ToggleGroupItem>
        </ToggleGroup>
      </Field>
      {mode === "grid" ? (
        <Field>
          <FieldLabel>Grid division</FieldLabel>
          <ToggleGroup
            value={[String(grid)]}
            disabled={busy}
            onValueChange={(value) => {
              if (value[0]) {
                reset()
                setGrid(Number(value[0]))
              }
            }}
          >
            {[PPQ / 4, PPQ / 2, PPQ, PPQ * 2, PPQ * 4].map((ticks, index) => (
              <ToggleGroupItem key={ticks} value={String(ticks)}>
                {["1/16", "1/8", "1/4", "1/2", "4 beats"][index]}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <FieldDescription>
            Note divisions aligned to the song, measured in quarter-note beats.
          </FieldDescription>
        </Field>
      ) : (
        <Field data-invalid={invalidSensitivity}>
          <FieldLabel htmlFor={id + "sensitivity"}>Sensitivity (%)</FieldLabel>
          <Input
            id={id + "sensitivity"}
            type="number"
            min="0"
            max="100"
            step="1"
            value={sensitivity}
            aria-invalid={invalidSensitivity}
            disabled={busy}
            onChange={(event) => {
              reset()
              setSensitivity(event.target.value)
            }}
          />
          <FieldDescription>
            Higher values include weaker energy attacks; review before applying.
          </FieldDescription>
          {invalidSensitivity && (
            <FieldError>Enter a sensitivity from 0 to 100.</FieldError>
          )}
        </Field>
      )}
      <Button
        variant="outline"
        disabled={busy || (mode === "transients" && invalidSensitivity)}
        onClick={() => void analyze()}
      >
        {busy ? "Working…" : "Analyze markers"}
      </Button>
      <FieldDescription>
        Tape clips support trim, offset, reverse, pitch, gain, pan, mute, and
        routing. Fades, spectral stretch, and project swing must be removed
        before slicing.
      </FieldDescription>
      {review && (
        <>
          <MarkerPreview review={review} selected={selected} />
          <FieldSet disabled={busy || stale}>
            <FieldLegend>Cut markers ({selected.length} selected)</FieldLegend>
            <FieldDescription>
              Positions are relative to the visible clip. Turn off any cut you
              do not want.
            </FieldDescription>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="outline"
                disabled={busy || stale}
                onClick={() =>
                  setSelected(review.analysis.markers.map((m) => m.tick))
                }
              >
                Select all
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={busy || stale}
                onClick={() => setSelected([])}
              >
                Clear markers
              </Button>
            </div>
            <FieldGroup className="max-h-44 overflow-y-auto">
              {review.analysis.markers.map((marker) => (
                <Field key={marker.tick} orientation="horizontal">
                  <Checkbox
                    id={id + marker.tick}
                    checked={selected.includes(marker.tick)}
                    disabled={busy || stale}
                    onCheckedChange={() => toggle(marker.tick)}
                  />
                  <FieldLabel htmlFor={id + marker.tick}>
                    Cut at {(marker.tick / PPQ).toFixed(3)} beats ({marker.tick}{" "}
                    ticks)
                  </FieldLabel>
                </Field>
              ))}
            </FieldGroup>
            {!review.analysis.markers.length && (
              <FieldDescription>
                No interior markers found. Try another grid or sensitivity.
              </FieldDescription>
            )}
          </FieldSet>
          <p role="status">
            {selected.length
              ? `${selected.length + 1} linked-source slices`
              : "No cuts selected"}
          </p>
        </>
      )}
      {stale && (
        <FieldError>The project changed. Analyze the clip again.</FieldError>
      )}
      {error && <FieldError>{error}</FieldError>}
      <Button
        disabled={busy || stale || !review || !selected.length}
        onClick={() => void apply()}
      >
        Apply slices
      </Button>
    </FieldGroup>
  )
}

function MarkerPreview({
  review,
  selected,
}: {
  review: SliceReview
  selected: number[]
}) {
  const height = 100
  const width = 640
  const peaks = review.analysis.peaks
  return (
    <svg
      role="img"
      aria-label={`Slice marker preview: ${selected.length} cuts`}
      viewBox={`0 0 ${width} ${height}`}
      className="h-28 w-full rounded-md border bg-muted"
    >
      <line
        x1={0}
        y1={height / 2}
        x2={width}
        y2={height / 2}
        className="stroke-muted-foreground"
        opacity={0.25}
      />
      {peaks.map((peak, index) => (
        <line
          key={index}
          x1={((index + 0.5) * width) / peaks.length}
          x2={((index + 0.5) * width) / peaks.length}
          y1={height / 2 - peak * 40}
          y2={height / 2 + peak * 40}
          className="stroke-muted-foreground"
          strokeWidth={3}
        />
      ))}
      {review.analysis.markers.map((marker) => (
        <line
          key={marker.tick}
          x1={(marker.tick / review.lengthTicks) * width}
          x2={(marker.tick / review.lengthTicks) * width}
          y1={0}
          y2={height}
          className="stroke-primary"
          strokeWidth={2}
          opacity={selected.includes(marker.tick) ? 1 : 0.2}
        />
      ))}
    </svg>
  )
}
