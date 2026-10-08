import { ValueContextItems } from "@/components/value-context-menu"
import { automationFeed, useAutomationMarker } from "@/features/automation/live"
import { trackValueItems } from "./menus"
import { useMemo } from "react"
import type { Send, TrackId } from "@/bindings"
import { faderTaper, gainUnit, Knob } from "@/components/audio"
import { Button } from "@/components/ui/button"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { dispatch, useProjectStore } from "@/lib/store"
import { MAX_GAIN } from "@/lib/units"
import { clampGain, removeSidechain } from "./operations"
import { outputChoices } from "./routing"
import { useGestureValue } from "./use-gesture-value"

function SidechainRow({ from, send }: { from: TrackId; send: Send }) {
  const target = useProjectStore((state) => state.project.mixer.tracks.find((track) => track.id === send.target)?.name ?? "Removed track")
  const level = useGestureValue(send.gain, (gain) => ({ type: "setSidechain", from, to: send.target, gain }), clampGain)
  const targetKey = { type: "sidechainGain", track: from, target: send.target } as const
  const live = useMemo(() => automationFeed(targetKey), [from, send.target])
  const marker = useAutomationMarker(targetKey)
  return <li className="flex items-center gap-2">
    <ValueContextItems items={trackValueItems(from, { sidechain: send.target })}><Knob live={live} marker={marker} size="sm" min={0} max={MAX_GAIN} scale={faderTaper} defaultValue={1} aria-label={`Sidechain level to ${target}`} {...gainUnit} {...level} /></ValueContextItems>
    <span className="min-w-0 flex-1 truncate text-xs">{target}</span>
    <Button size="sm" variant="ghost" aria-label={`Remove sidechain to ${target}`} onClick={() => void removeSidechain(from, send.target)}>×</Button>
  </li>
}

export function SidechainPanel({ track }: { track: TrackId }) {
  const tracks = useProjectStore((state) => state.project.mixer.tracks)
  const source = tracks.find((item) => item.id === track)
  const sends = source?.sidechains ?? []
  const incoming = tracks.filter((item) => item.sidechains?.some((send) => send.target === track))
  const choices = useMemo(() => outputChoices(tracks, track).tracks.filter((item) => !sends.some((send) => send.target === item.id)), [tracks, track, sends])
  if (!source || source.current) return null
  return <div className="shrink-0 border-b p-1.5"><Popover>
    <PopoverTrigger render={<Button variant="outline" size="sm" className="w-full justify-start">Sidechain · {incoming.length} in / {sends.length} out</Button>} />
    <PopoverContent align="start" className="w-80 space-y-3">
      <p className="text-xs text-muted-foreground">Detector-only audio is kept separate from the destination's audible input. Enable External sidechain on its compressor to use the combined key bus.</p>
      <p className="text-xs font-medium">Detector inputs</p>
      {incoming.length ? <ul className="max-h-36 space-y-1 overflow-y-auto">{incoming.map((item) => <li key={item.id} className="flex items-center gap-1 text-xs"><span className="min-w-0 flex-1 truncate">{item.name}</span><Button size="sm" variant="ghost" aria-label={`Remove detector input from ${item.name}`} onClick={() => void removeSidechain(item.id, track)}>×</Button></li>)}</ul> : <p className="text-xs text-muted-foreground">No external detector signal.</p>}
      {track !== 0 && <>
        <p className="text-xs font-medium">Send to detectors</p>
        <ul className="max-h-48 space-y-1 overflow-y-auto">{sends.map((send) => <SidechainRow key={send.target} from={track} send={send} />)}</ul>
        <Select value={null} disabled={!choices.length} items={choices.map((item) => ({ value: item.id, label: item.name }))} onValueChange={(to: number | null) => { if (to !== null) void dispatch({ type: "setSidechain", from: track, to, gain: 1 }) }}>
          <SelectTrigger aria-label="Add detector-only sidechain"><SelectValue placeholder="Add sidechain destination…" /></SelectTrigger>
          <SelectContent><SelectGroup>{choices.map((item) => <SelectItem key={item.id} value={item.id}>{item.name}</SelectItem>)}</SelectGroup></SelectContent>
        </Select>
      </>}
    </PopoverContent>
  </Popover></div>
}
