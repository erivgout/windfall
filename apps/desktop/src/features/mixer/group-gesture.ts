import { useRef } from "react"
import type { MixerTrack, TrackId } from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { MAX_GAIN } from "@/lib/units"
import { selectedMixerTracks } from "./mixer-ui"
import { useGestureValue } from "./use-gesture-value"

export function useTrackGroupGesture(id: TrackId, field: "volume" | "pan") {
  const value = useProjectStore((state) => state.project.mixer.tracks.find((track) => track.id === id)?.[field] ?? (field === "volume" ? 1 : 0))
  const captured = useRef<MixerTrack[] | null>(null)
  const initial = useRef(value)
  const clamp = (value: number) => Math.max(field === "volume" ? 0 : -1, Math.min(field === "volume" ? MAX_GAIN : 1, value))
  const gesture = useGestureValue(value, (next) => {
    const tracks = captured.current ?? selectedMixerTracks(id)
    const base = captured.current ? initial.current : value
    return { type: "batch", commands: tracks.map((track) => ({ type: "updateMixerTrack", id: track.id, patch: { [field]: clamp(field === "pan" ? track.pan + next - base : base > 0 ? track.volume * next / base : next) } })) }
  }, clamp)
  return { ...gesture, onGestureStart() { captured.current = selectedMixerTracks(id).map((track) => structuredClone(track)); initial.current = value; gesture.onGestureStart() }, onGestureEnd() { gesture.onGestureEnd(); captured.current = null } }
}
