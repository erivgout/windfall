import type { Command } from "@/bindings"
import type { ArrangementEdit } from "./model"

/** Added metadata IDs are assigned by Rust, never by the panel's advisory allocator. */
export function arrangementCommand(edit: ArrangementEdit): Command {
  switch (edit.type) {
    case "addArrangement":
      return {
        type: "addArrangement",
        name: edit.arrangement.name,
        clips: edit.arrangement.clips,
        tracks: edit.arrangement.tracks,
      }
    case "setReferences":
      return {
        type: "setArrangementReferences",
        id: edit.id,
        clips: edit.clips,
        tracks: edit.tracks,
      }
    case "addTrackGroup":
      return { type: "addTrackGroup", name: edit.group.name, parent: null }
    case "moveTrack":
      return { type: "moveTrackToGroup", track: edit.id, parent: edit.parent }
    case "addClipGroup":
      return { type: "addClipGroup", clips: edit.group.clips }
    case "linkTrack":
      return { type: "linkTrack", track: edit.id, kind: edit.kind }
    default:
      return edit
  }
}
