import type { ChannelId } from "@/bindings"
import { reportError } from "@/lib/errors"
import { backend } from "@/lib/ipc"

import { logAuditionOff, logAuditionOn } from "./note-log"

export function auditionOn(channel: ChannelId, key: number, velocity: number) {
  logAuditionOn(channel, key, velocity)
  backend
    .auditionNoteOn(channel, key, velocity)
    .catch((error: unknown) => reportError(error, "Could not play the note"))
}

export function auditionOff(channel: ChannelId, key: number) {
  logAuditionOff(channel, key)
  backend
    .auditionNoteOff(channel, key)
    .catch((error: unknown) => reportError(error, "Could not stop the note"))
}
