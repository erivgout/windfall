import { useState } from "react"
import type { ExternalOutputRoute, TrackId } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { useProjectStore, dispatch } from "@/lib/store/project"
import { useEngineStore } from "@/lib/store/engine"
import { useRecordingStore } from "@/features/transport/recording-store"
import { errorMessage } from "@/lib/ipc"

export function ExternalOutputPanel({ track }: { track: TrackId }) {
  const route = useProjectStore((state) => state.project.mixer.tracks.find((item) => item.id === track)?.externalOutput)
  const status = useEngineStore((state) => state.status)
  const recording = useRecordingStore((state) => state.state.active || state.busy)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState("")
  const channels = status?.running ? status.outputChannels ?? 2 : 0
  const unavailable = !!route && (route.left >= channels || route.right !== null && route.right >= channels)
  const extent = Math.min(256, Math.max(channels, (route?.left ?? 0) + 1, (route?.right ?? 0) + 1))
  const ports = Array.from({ length: extent }, (_, value) => ({ value, label: `Output ${value + 1}${value >= channels ? " (unavailable)" : ""}` }))
  const change = async (route: ExternalOutputRoute | null) => {
    if (busy) return
    setBusy(true); setError("")
    try { if (!await dispatch({ type: "setTrackExternalOutput", id: track, route })) setError("The output assignment was not applied.") }
    catch (error) { setError(errorMessage(error)) }
    finally { setBusy(false) }
  }
  const patch = (patch: Partial<ExternalOutputRoute>) => { if (route) void change({ ...route, ...patch }) }
  const disabled = recording || busy
  const label = route ? `Out ${route.left + 1}${route.right === null ? " mono" : ` / ${route.right + 1}`}` : track === 0 ? "Default device output" : "No hardware output"
  return <div className="shrink-0 border-b p-1.5"><Popover>
    <PopoverTrigger render={<Button variant="outline" size="sm" className={`w-full justify-start truncate ${unavailable ? "text-destructive" : ""}`}>{label}</Button>} />
    <PopoverContent align="start" className="w-80"><FieldGroup>
      <Field orientation="horizontal"><Checkbox id={`external-enable-${track}`} checked={!!route} disabled={disabled || !channels && !route} onCheckedChange={(enabled) => void change(enabled === true ? { left: 0, right: channels > 1 ? 1 : null, exclusive: track !== 0 } : null)} /><FieldLabel htmlFor={`external-enable-${track}`}>Assign hardware output</FieldLabel></Field>
      <FieldDescription>{status?.running ? `${status.device ?? "Default device"}: ${channels} open output ports.` : "Open an output device in Audio settings."} Choose a wider output channel layout there to expose more interface ports.</FieldDescription>
      {route && <>
        <Field><FieldLabel>Left / mono port</FieldLabel><Select items={ports} value={route.left} disabled={disabled} onValueChange={(left: number | null) => { if (left !== null) patch({ left, right: left === route.right ? null : route.right }) }}><SelectTrigger aria-label="Left hardware output"><SelectValue /></SelectTrigger><SelectContent><SelectGroup>{ports.map((port) => <SelectItem key={port.value} value={port.value}>{port.label}</SelectItem>)}</SelectGroup></SelectContent></Select></Field>
        <Field><FieldLabel>Right port</FieldLabel><Select items={[{ value: -1, label: "Mono fold" }, ...ports.filter((port) => port.value !== route.left)]} value={route.right ?? -1} disabled={disabled} onValueChange={(right: number | null) => { if (right !== null) patch({ right: right < 0 ? null : right }) }}><SelectTrigger aria-label="Right hardware output"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem value={-1}>Mono fold</SelectItem>{ports.filter((port) => port.value !== route.left).map((port) => <SelectItem key={port.value} value={port.value}>{port.label}</SelectItem>)}</SelectGroup></SelectContent></Select></Field>
        {track !== 0 && <Field orientation="horizontal"><Checkbox id={`external-exclusive-${track}`} checked={route.exclusive} disabled={disabled} onCheckedChange={(exclusive) => patch({ exclusive: exclusive === true })} /><FieldLabel htmlFor={`external-exclusive-${track}`}>Replace the ordinary mixer output</FieldLabel></Field>}
      </>}
      <FieldDescription>{track === 0 ? "The completed Master mix, printed clips, previews and metronome feed this port assignment." : "Hardware receives this track after effects, fader and pan. Explicit mixer sends still run; turn replacement off to also feed its ordinary mixer output."}</FieldDescription>
      {unavailable && <p role="alert" className="text-destructive">This saved port pair is unavailable in the open layout. Its hardware path is silent until those ports are opened or reassigned.</p>}
      {error && <p role="alert" className="text-destructive">{error}</p>}
    </FieldGroup></PopoverContent>
  </Popover></div>
}
