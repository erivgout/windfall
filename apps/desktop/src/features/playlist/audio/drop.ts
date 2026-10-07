import { backend } from "@/lib/ipc"

import { wholeClipTicks } from "./geometry"

/*
 * How long a file dragged over the timeline will be as a clip, so the drop
 * preview has its real length. The file's length is read once per path.
 */

const MAX_KEPT = 200

/** Seconds by path. Null while a file is being read, and for one that cannot be. */
const durations = new Map<string, number | null>()

/**
 * The length of a clip of the whole file at a tempo, in ticks, or null
 * while the file's length is not known. The first call for a path starts
 * reading it; `onKnown` is called when the answer is there.
 */
export function fileClipTicks(
  path: string,
  tempoBpm: number,
  onKnown?: () => void
): number | null {
  const known = durations.get(path)
  if (known === undefined) {
    if (durations.size >= MAX_KEPT) durations.clear()
    durations.set(path, null)
    void backend.sampleInfo(path).then(
      (info) => {
        durations.set(path, info.durationSecs)
        onKnown?.()
      },
      // An unreadable file is refused when it is dropped, with the reason.
      () => undefined
    )
    return null
  }
  return known === null ? null : wholeClipTicks(known, tempoBpm)
}
