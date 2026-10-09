import {
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"

import { toolActionId } from "./actions"
import { TOOLS } from "./intents"

/**
 * Everywhere in the playlist that has no menu of its own: the toolbar, the
 * pattern list around its patterns, the scrollbars and the corners. The
 * clips, the ruler and the track names each have theirs.
 */
export const PANEL_MENU: ContextItem[] = [
  { submenu: "Tool", items: TOOLS.map(toolActionId) },
  contextSeparator,
  "playlist.paste",
  "playlist.selectAll",
  "playlist.selectMatchingClips",
  "playlist.selectAtPlayhead",
  "playlist.selectMutedClips",
  "playlist.selectClipsOnMutedTracks",
  "playlist.makeUnique",
  "playlist.bounceSelectedClips",
  "playlist.addTrack",
  contextSeparator,
  "playlist.zoomToFit",
  "playlist.follow",
  "playlist.loopSong",
  "playlist.patterns",
]
