import { useState } from "react"
import { create } from "zustand"
import type { MixerTrackPreset, TrackId } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Input } from "@/components/ui/input"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { backend, errorMessage } from "@/lib/ipc"
import { dispatch, useProjectStore } from "@/lib/store"
import { getProjectGeneration } from "@/lib/store/replaced"

type SavedPreset = { key: string; preset: MixerTrackPreset }
const STORAGE_KEY = "windfall.mixer-presets.v1"
function readShelf(): SavedPreset[] {
  try {
    const data: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]")
    return Array.isArray(data) ? data.filter((item) => item && typeof item.key === "string" && item.preset?.version === 1 && Array.isArray(item.preset.effects)).slice(0, 128) : []
  } catch { return [] }
}
const useShelf = create<{ entries: SavedPreset[] }>(() => ({ entries: readShelf() }))
function writeShelf(entries: SavedPreset[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(entries))
  useShelf.setState({ entries })
}

export function TrackPresetsPanel({ track }: { track: TrackId }) {
  const trackName = useProjectStore((state) => state.project.mixer.tracks.find((item) => item.id === track)?.name ?? "Mixer track")
  const entries = useShelf((state) => state.entries)
  const [name, setName] = useState(trackName)
  const [nameColor, setNameColor] = useState(false)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState("")
  async function capture() {
    const generation = getProjectGeneration()
    const guard = await backend.timelineState()
    const preset = await backend.mixerPresetCapture(track, guard.generation, guard.revision)
    if (getProjectGeneration() !== generation) throw new Error("The project changed during preset capture")
    return { ...preset, name: name.trim() || trackName }
  }
  async function run(work: () => Promise<void>) {
    if (busy) return
    setBusy(true); setMessage("")
    try { await work() } catch (error) { setMessage(errorMessage(error)) } finally { setBusy(false) }
  }
  async function apply(preset: MixerTrackPreset, generation: number, expected: import("@/bindings").MixerTrack) {
    if (getProjectGeneration() !== generation) throw new Error("The project changed before loading the preset")
    if (await dispatch({ type: "applyMixerTrackPreset", id: track, expected, preset, nameColor })) setMessage(`Loaded ${preset.name}`)
    else setMessage("Preset was not applied. The destination may have changed.")
  }
  return <div className="shrink-0 border-b p-1.5"><Popover>
    <PopoverTrigger render={<Button size="sm" variant="outline" className="w-full justify-start">Track presets</Button>} />
    <PopoverContent align="start" className="w-80 space-y-3">
      <Input aria-label="Mixer preset name" value={name} onChange={(event) => setName(event.target.value)} disabled={busy} maxLength={120} />
      <div className="flex gap-1">
        <Button size="sm" variant="outline" disabled={busy || entries.length >= 128} onClick={() => void run(async () => {
          const preset = await capture()
          writeShelf([...useShelf.getState().entries, { key: crypto.randomUUID(), preset }])
          setMessage(`Saved ${preset.name} to the preset shelf`)
        })}>Save to shelf</Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => void run(async () => {
          const path = await backend.mixerPresetSave(await capture())
          if (path) setMessage(`Saved ${path}`)
        })}>Save file…</Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => void run(async () => {
          const generation = getProjectGeneration()
          const expected = structuredClone(useProjectStore.getState().project.mixer.tracks.find((item) => item.id === track))
          if (!expected) throw new Error("The destination track no longer exists")
          const preset = await backend.mixerPresetLoad()
          if (preset) await apply(preset, generation, expected)
        })}>Load file…</Button>
      </div>
      <label className="flex items-center gap-2 text-xs"><Checkbox checked={nameColor} onCheckedChange={(checked) => setNameColor(checked === true)} disabled={busy} />Restore preset name and color</label>
      <p className="text-xs text-muted-foreground">Restores effects, their saved plugin state, fader, pan, mute, EQ, stereo controls and latency correction. Existing effect automation is removed. Routing, recording, dock and hardware assignments stay with this track.</p>
      <ul aria-label="Saved mixer presets" className="max-h-48 space-y-1 overflow-y-auto">
        {entries.map((entry) => <li key={entry.key} className="flex items-center gap-1">
          <Button size="sm" variant="ghost" className="min-w-0 flex-1 justify-start truncate" disabled={busy} onClick={() => void run(async () => {
            const expected = structuredClone(useProjectStore.getState().project.mixer.tracks.find((item) => item.id === track))
            if (!expected) throw new Error("The destination track no longer exists")
            await apply(entry.preset, getProjectGeneration(), expected)
          })}>{entry.preset.name} · {entry.preset.effects.length} FX</Button>
          <Button size="sm" variant="ghost" aria-label={`Delete preset ${entry.preset.name}`} disabled={busy} onClick={() => void run(async () => writeShelf(useShelf.getState().entries.filter((item) => item.key !== entry.key)))}>×</Button>
        </li>)}
      </ul>
      {message && <p role="status" className="break-words text-xs">{message}</p>}
    </PopoverContent>
  </Popover></div>
}
