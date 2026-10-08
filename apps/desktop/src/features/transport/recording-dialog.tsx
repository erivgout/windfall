import { useEffect } from "react"
import { ActionButton } from "@/components/action-button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { useProjectStore } from "@/lib/store/project"
import { setTransport, useTransportStore } from "@/lib/store/transport"
import { refreshRecording, useRecordingStore } from "./recording-store"
function Choice({
  label,
  value,
  items,
  onChange,
  disabled,
}: {
  label: string
  value: number
  items: { value: number; label: string }[]
  onChange: (value: number) => void
  disabled: boolean
}) {
  return (
    <Select
      items={items}
      value={value}
      disabled={disabled}
      onValueChange={(v: number | null) => {
        if (v !== null) onChange(v)
      }}
    >
      <SelectTrigger aria-label={label}>
        <SelectValue placeholder="Not available" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {items.map((i) => (
            <SelectItem key={i.value} value={i.value}>
              {i.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}
export function RecordingDialog() {
  const countInBars = useTransportStore((state) => state.countInBars ?? 0)
  const countInRemaining = useTransportStore((state) => state.countInRemaining ?? 0)
  const s = useRecordingStore()
  const tracks = useProjectStore((p) => p.project.playlist.tracks)
  const mixerTracks = useProjectStore((p) => p.project.mixer.tracks)
  useEffect(() => {
    const timer = setInterval(() => {
      void refreshRecording()
    }, 500)
    return () => clearInterval(timer)
  }, [])
  const disabled = s.busy || s.state.active
  const input = s.inputs[s.device]
  const channels = Array.from({ length: input?.channels ?? 0 }, (_, value) => ({
    value,
    label: `Input ${value + 1}`,
  }))
  return (
    <Dialog
      open={s.open}
      onOpenChange={(open) => useRecordingStore.setState({ open })}
    >
      <DialogContent className="max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Record audio input</DialogTitle>
          <DialogDescription>
            Record a mono or stereo input into the playlist. Synchronize playback
            to the chosen start, use a count-in, and set an offset for your setup.
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field orientation="horizontal">
            <Checkbox id="record-armed" checked={s.armedTracks} disabled={disabled} onCheckedChange={(checked) => useRecordingStore.setState({ armedTracks: checked === true })} />
            <FieldLabel htmlFor="record-armed">Record all armed mixer tracks</FieldLabel>
          </Field>
          {s.armedTracks && <FieldDescription>Choose dry inputs or processed track sound and arm tracks in the mixer inspector. Each armed source gets its own aligned recording. {mixerTracks.filter((track) => track.recording?.armed).length} tracks armed.</FieldDescription>}
          <Field><FieldLabel>Count-in</FieldLabel><Choice label="Count-in bars" value={countInBars} disabled={disabled} items={[{ value: 0, label: "None" }, { value: 1, label: "1 bar" }, { value: 2, label: "2 bars" }, { value: 4, label: "4 bars" }, { value: 8, label: "8 bars" }]} onChange={(countInBars) => void setTransport({ countInBars })} /><FieldDescription>Count-in follows the song meter and tempo at the chosen start. Playback begins there after the clicks.</FieldDescription></Field>
          <Field orientation="horizontal">
            <Checkbox id="record-sync" checked={s.synchronize || s.loopRecording || s.armedTracks || countInBars > 0} disabled={disabled || countInBars > 0 || s.loopRecording || s.armedTracks} onCheckedChange={(checked) => useRecordingStore.setState({ synchronize: checked === true })} />
            <FieldLabel htmlFor="record-sync">Start playback with recording</FieldLabel>
          </Field>
          <Field>
            <FieldLabel>Input device</FieldLabel>
            <Choice
              label="Input device"
              value={s.device}
              disabled={disabled || s.armedTracks}
              items={s.inputs.map((d, value) => ({
                value,
                label: `${d.host}: ${d.device}`,
              }))}
              onChange={(device) =>
                useRecordingStore.setState({ device, left: 0, right: -1, inputRate: 0 })
              }
            />
            <FieldDescription>
              Input opens before playback. Mono is stored as duplicated stereo.
            </FieldDescription>
          </Field>
          <Field>
            <FieldLabel>Input sample rate</FieldLabel>
            <Choice label="Input sample rate" value={s.inputRate} disabled={disabled} items={[{ value: 0, label: "Automatic" }, ...(input?.sampleRates ?? []).map((value) => ({ value, label: `${value} Hz` }))]} onChange={(inputRate) => useRecordingStore.setState({ inputRate })} />
            <FieldDescription>Audio is converted to the output sample rate when needed.</FieldDescription>
          </Field>
          <Field orientation="horizontal">
            <Checkbox id="record-drift" checked={s.driftCorrection} disabled={disabled} onCheckedChange={(checked) => useRecordingStore.setState({ driftCorrection: checked === true })} />
            <FieldLabel htmlFor="record-drift">Correct independent device clock drift</FieldLabel>
          </Field>
          <Field>
            <FieldLabel htmlFor="record-offset">Recording offset (milliseconds)</FieldLabel>
            <Input id="record-offset" type="number" min={-1000} max={1000} step={0.1} value={s.offsetMs} disabled={disabled} onChange={(event) => useRecordingStore.setState({ offsetMs: Math.max(-1000, Math.min(1000, Number(event.target.value))) })} />
            <FieldDescription>Positive values advance captured audio. Driver timestamps supply automatic input/output latency alignment.</FieldDescription>
          </Field>
          <Field>
            <FieldLabel>Left or mono channel</FieldLabel>
            <Choice
              label="Left or mono channel"
              value={s.left}
              disabled={disabled || s.armedTracks}
              items={channels}
              onChange={(left) => useRecordingStore.setState({ left })}
            />
          </Field>
          <Field>
            <FieldLabel>Right channel</FieldLabel>
            <Choice
              label="Right channel"
              value={s.right}
              disabled={disabled || s.armedTracks}
              items={[
                { value: -1, label: "Mono (duplicate left)" },
                ...channels,
              ]}
              onChange={(right) => useRecordingStore.setState({ right })}
            />
          </Field>
          <Field>
            <FieldLabel htmlFor="record-start">Playlist start tick</FieldLabel>
            <Input
              id="record-start"
              type="number"
              min={0}
              step={1}
              value={s.start}
              disabled={disabled}
              onChange={(e) =>
                useRecordingStore.setState({
                  start: Math.max(0, Math.floor(Number(e.target.value))),
                })
              }
            />
          </Field>
          <Field>
            <FieldLabel>Playlist track</FieldLabel>
            <Choice
              label="Playlist track"
              value={s.track}
              disabled={disabled}
              items={[
                { value: -1, label: "New recording track" },
                ...tracks.map((t) => ({ value: t.id, label: t.name })),
              ]}
              onChange={(track) => useRecordingStore.setState({ track })}
            />
          </Field>
          <Field orientation="horizontal">
            <Checkbox id="record-loop" checked={s.loopRecording} disabled={disabled} onCheckedChange={(checked) => useRecordingStore.setState({ loopRecording: checked === true })} />
            <FieldLabel htmlFor="record-loop">Record loop takes</FieldLabel>
          </Field>
          {s.loopRecording && <>
            <Field>
              <FieldLabel htmlFor="record-loop-end">Loop end tick</FieldLabel>
              <Input id="record-loop-end" type="number" min={s.start + 1} step={1} value={s.loopEnd} disabled={disabled} onChange={(event) => useRecordingStore.setState({ loopEnd: Math.max(s.start + 1, Math.floor(Number(event.target.value))) })} />
              <FieldDescription>Each pass from the start tick to this end becomes a take. The final partial pass is retained too.</FieldDescription>
            </Field>
            <Field orientation="horizontal">
              <Checkbox id="record-latest" checked={s.keepLatest} disabled={s.busy} onCheckedChange={(checked) => useRecordingStore.setState({ keepLatest: checked === true })} />
              <FieldLabel htmlFor="record-latest">Keep only the latest take</FieldLabel>
            </Field>
          </>}
          <Field orientation="horizontal">
            <Checkbox id="record-monitor" checked={s.monitor && !s.armedTracks} disabled={disabled || s.armedTracks} onCheckedChange={(checked) => useRecordingStore.setState({ monitor: checked === true })} />
            <FieldLabel htmlFor="record-monitor">Monitor input through Windfall</FieldLabel>
          </Field>
          {s.monitor && !s.armedTracks && <>
            <Field>
              <FieldLabel>Monitor mixer track</FieldLabel>
              <Choice label="Monitor mixer track" value={s.monitorTrack} disabled={disabled} items={mixerTracks.map((track) => ({ value: track.id, label: track.name }))} onChange={(monitorTrack) => useRecordingStore.setState({ monitorTrack })} />
              <FieldDescription>Use headphones to keep the output from feeding back into the microphone. Monitor effects are heard while the recording keeps the original input.</FieldDescription>
            </Field>
            <Field>
              <FieldLabel htmlFor="record-monitor-gain">Monitor level (%)</FieldLabel>
              <Input id="record-monitor-gain" type="number" min={0} max={100} step={1} value={Math.round(s.monitorGain * 100)} disabled={disabled} onChange={(event) => useRecordingStore.setState({ monitorGain: Math.max(0, Math.min(1, Number(event.target.value) / 100)) })} />
            </Field>
            <Field>
              <FieldLabel htmlFor="record-monitor-buffer">Monitor buffer (milliseconds)</FieldLabel>
              <Input id="record-monitor-buffer" type="number" min={5} max={100} step={1} value={s.monitorBufferMs} disabled={disabled} onChange={(event) => useRecordingStore.setState({ monitorBufferMs: Math.max(5, Math.min(100, Math.round(Number(event.target.value)))) })} />
              <FieldDescription>A larger buffer accommodates driver scheduling delays and adds monitoring latency.</FieldDescription>
            </Field>
          </>}
        </FieldGroup>
        {s.inputs.length === 0 && !s.armedTracks && (
          <p>
            No audio inputs found. Recording needs the native desktop app and an
            available input device.
          </p>
        )}
        <p role="status">
          {s.state.active
            ? countInRemaining > 0 ? `Count-in: ${countInRemaining} beats remaining.` : `Recording ${(s.state.frames / Math.max(1, s.state.sampleRate)).toFixed(1)} seconds at ${s.state.sampleRate} Hz. Stop and keep, or discard.`
            : "Ready. Recording sources are kept in Windfall’s recordings folder and copied into the project on save."}
        </p>
        {s.state.active && s.state.alignment && <p className="text-sm text-muted-foreground">
          Input {s.state.alignment.inputSampleRate} Hz · input latency {s.state.alignment.inputLatencyMs.toFixed(1)} ms · output latency {s.state.alignment.outputLatencyMs.toFixed(1)} ms · drift {s.state.alignment.driftPpm.toFixed(1)} ppm.
          {s.state.alignment.measuredTimestamps ? " Driver timestamps available." : " Timestamp estimate in use."}
        </p>}
        {s.state.active && s.state.takes && s.state.takes.length > 0 && <fieldset className="max-h-40 space-y-2 overflow-y-auto rounded border p-3">
          <legend className="px-1 text-sm">Takes to keep</legend>
          {s.state.takes.map((take, index, takes) => <Field key={take.index} orientation="horizontal">
            <Checkbox id={`record-take-${take.index}`} checked={s.keepLatest ? index === takes.length - 1 : !s.excludedTakes.includes(take.index)} disabled={s.busy || s.keepLatest} onCheckedChange={(checked) => useRecordingStore.setState({ excludedTakes: checked === true ? s.excludedTakes.filter((value) => value !== take.index) : [...s.excludedTakes, take.index] })} />
            <FieldLabel htmlFor={`record-take-${take.index}`}>Take {take.index + 1} · {(take.frames / Math.max(1, s.state.sampleRate)).toFixed(1)} s{take.complete ? "" : " (recording)"}</FieldLabel>
          </Field>)}
          <p className="text-xs text-muted-foreground">Kept takes share the loop start. The latest is audible; earlier takes are muted for auditioning in the playlist.</p>
        </fieldset>}
        {s.state.active && s.state.monitor && <p className="text-sm text-muted-foreground">Monitor buffer {s.state.monitor.bufferedMs.toFixed(1)} ms · dropped {s.state.monitor.droppedFrames} frames · starved {s.state.monitor.starvedFrames} frames.</p>}
        {s.state.active && s.state.tracks && <ul aria-label="Recording tracks" className="max-h-40 space-y-1 overflow-y-auto text-xs">
          {s.state.tracks.map((track) => <li key={track.mixerTrack} className="rounded border p-2">
            <p className="font-medium">{track.name} · {(track.frames / Math.max(1, s.state.sampleRate)).toFixed(1)} s · {mixerTracks.find((item) => item.id === track.mixerTrack)?.recording?.mode === "postEffects" ? "after effects" : mixerTracks.find((item) => item.id === track.mixerTrack)?.recording?.mode === "postFader" ? "after fader" : "dry input"}</p>
            <p className="text-muted-foreground">Source {track.alignment.inputSampleRate} Hz · latency {track.alignment.inputLatencyMs.toFixed(1)} ms · drift {track.alignment.driftPpm.toFixed(1)} ppm{track.monitor ? ` · monitor ${track.monitor.bufferedMs.toFixed(1)} ms / ${track.monitor.droppedFrames} dropped frames` : ""}</p>
          </li>)}
        </ul>}
        {(s.error || s.state.error) && (
          <p role="alert" className="text-destructive">
            {s.error || s.state.error}
          </p>
        )}
        <div className="flex gap-2">
          <ActionButton action="recording.start">Start recording</ActionButton>
          <ActionButton action="recording.stop" variant="secondary">
            Stop and keep
          </ActionButton>
          <ActionButton action="recording.cancel" variant="outline">
            Discard
          </ActionButton>
        </div>
      </DialogContent>
    </Dialog>
  )
}
