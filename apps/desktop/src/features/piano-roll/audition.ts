import type { ChannelId } from "@/bindings"
import { reportError } from "@/lib/errors"
import { backend } from "@/lib/ipc"

export function auditionOn(channel: ChannelId, key: number, velocity: number) {
  backend
    .auditionNoteOn(channel, key, velocity)
    .catch((error: unknown) => reportError(error, "Could not play the note"))
}

export function auditionOff(channel: ChannelId, key: number) {
  backend
    .auditionNoteOff(channel, key)
    .catch((error: unknown) => reportError(error, "Could not stop the note"))
}
