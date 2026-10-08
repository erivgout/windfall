import { useEffect, useState } from "react"
import type { TrackId } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { useProjectStore, dispatch } from "@/lib/store/project"
import { useRecordingStore } from "@/features/transport/recording-store"
import { errorMessage } from "@/lib/ipc"

export function LatencyOffsetPanel({ track }: { track: TrackId }) {
  const value = useProjectStore((state) => state.project.mixer.tracks.find((item) => item.id === track)?.latencyOffsetMs ?? 0)
  const recording = useRecordingStore((state) => state.state.active || state.busy)
  const [draft, setDraft] = useState(String(value))
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState("")
  useEffect(() => { setDraft(String(value)); setError("") }, [track, value])
  const apply = async (offset: number, reset = false) => {
    if (busy || recording) return
    if (!Number.isFinite(offset) || Math.abs(offset) > 1000 || !reset && !draft.trim()) {
      setError("Enter a correction between -1000 and 1000 ms."); return
    }
    setBusy(true); setError("")
    try {
      if (!await dispatch({ type: "updateMixerTrack", id: track, patch: { latencyOffsetMs: offset } })) setError("The correction was not applied.")
      else setDraft(String(offset))
    } catch (error) { setError(errorMessage(error)) }
    finally { setBusy(false) }
  }
  return <div className="shrink-0 border-b p-1.5"><Popover>
    <PopoverTrigger render={<Button variant="outline" size="sm" className="w-full justify-start truncate">Latency correction: {value > 0 ? "+" : ""}{value} ms</Button>} />
    <PopoverContent align="start" className="w-80"><FieldGroup>
      <Field><FieldLabel htmlFor={`latency-offset-${track}`}>Declared latency correction (ms)</FieldLabel>
        <Input id={`latency-offset-${track}`} type="number" min={-1000} max={1000} step={0.01} value={draft} disabled={recording || busy} onChange={(event) => setDraft(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") void apply(Number(draft)); if (event.key === "Escape") { setDraft(String(value)); setError("") } }} />
        <FieldDescription>Positive values account for delay that a plugin or external signal does not report. Negative values reduce an overstated report. The engine compensates other paths using the corrected declaration.</FieldDescription>
      </Field>
      <FieldDescription>The declaration is rounded to device samples and cannot become negative. Use the recording offset control to move captured takes. This correction changes alignment on playback and export.</FieldDescription>
      <div className="flex gap-2"><Button size="sm" disabled={recording || busy} onClick={() => void apply(Number(draft))}>Apply</Button><Button size="sm" variant="outline" disabled={recording || busy || value === 0} onClick={() => void apply(0, true)}>Reset</Button></div>
      {error && <p role="alert" className="text-destructive">{error}</p>}
    </FieldGroup></PopoverContent>
  </Popover></div>
}
