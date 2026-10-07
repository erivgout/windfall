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
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { useProjectStore } from "@/lib/store/project"
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
  const s = useRecordingStore()
  const tracks = useProjectStore((p) => p.project.playlist.tracks)
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
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Record audio input</DialogTitle>
          <DialogDescription>
            Start playback separately, then record. Capture begins at the first
            input callback and is placed at the tick you choose. Hardware
            input/output latency and independent device clocks are not
            compensated; adjust the finished clip by ear.
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel>Input device</FieldLabel>
            <Choice
              label="Input device"
              value={s.device}
              disabled={disabled}
              items={s.inputs.map((d, value) => ({
                value,
                label: `${d.host}: ${d.device}`,
              }))}
              onChange={(device) =>
                useRecordingStore.setState({ device, left: 0, right: -1 })
              }
            />
            <FieldDescription>
              Input must support the output sample rate. Mono is stored as
              duplicated stereo; no live monitoring.
            </FieldDescription>
          </Field>
          <Field>
            <FieldLabel>Left or mono channel</FieldLabel>
            <Choice
              label="Left or mono channel"
              value={s.left}
              disabled={disabled}
              items={channels}
              onChange={(left) => useRecordingStore.setState({ left })}
            />
          </Field>
          <Field>
            <FieldLabel>Right channel</FieldLabel>
            <Choice
              label="Right channel"
              value={s.right}
              disabled={disabled}
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
        </FieldGroup>
        {s.inputs.length === 0 && (
          <p>
            No audio inputs found. Recording needs the native desktop app and an
            available input device.
          </p>
        )}
        <p role="status">
          {s.state.active
            ? `Recording ${(s.state.frames / Math.max(1, s.state.sampleRate)).toFixed(1)} seconds at ${s.state.sampleRate} Hz. Stop and keep, or discard.`
            : "Ready. Recording sources are kept in Windfall’s recordings folder and copied into the project on save."}
        </p>
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
