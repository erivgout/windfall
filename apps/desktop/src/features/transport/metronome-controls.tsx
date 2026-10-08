import { ActionButton } from "@/components/action-button"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { DEFAULT_METRONOME, setMetronome, useTransportStore } from "@/lib/store/transport"
import { useRecordingStore } from "./recording-store"

export function MetronomeControls() {
  const settings = useTransportStore((state) => state.metronome ?? DEFAULT_METRONOME)
  const recording = useRecordingStore((state) => state.state.active)
  return <div className="flex items-center gap-0.5">
    <ActionButton action="transport.metronome" variant={settings.enabled ? "secondary" : "ghost"} size="sm" aria-pressed={settings.enabled}>Click</ActionButton>
    <Popover><PopoverTrigger render={<Button variant="ghost" size="sm" aria-label="Metronome settings">⋯</Button>} /><PopoverContent className="w-64 space-y-3">
      <p className="text-sm font-medium">Metronome</p>
      <label className="block space-y-1 text-xs">Click volume (%)<Input type="number" min={0} max={100} value={Math.round(settings.gain * 100)} disabled={recording} onChange={(event) => { const value = Number(event.target.value); if (event.target.value && Number.isFinite(value) && value >= 0 && value <= 100) void setMetronome({ gain: value / 100 }) }} /></label>
      <Button variant={settings.accent ? "secondary" : "outline"} size="sm" aria-pressed={settings.accent} disabled={recording} onClick={() => void setMetronome({ accent: !settings.accent })}>Accent the first beat</Button>
    </PopoverContent></Popover>
  </div>
}
