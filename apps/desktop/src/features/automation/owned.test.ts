import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { AutomationTarget } from "@/bindings"
import { deleteChannel } from "@/features/channel-rack/channel-ops"
import { removeEffect, replaceEffect } from "@/features/mixer/effect-ops"
import { deleteTrack, removeSend } from "@/features/mixer/operations"
import type { MockBackend } from "@/lib/ipc/mock"
import { dispatch, undo, useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { settle, startTestApp } from "@/test/harness"

import { automationOwnedBy, ownedSentence, type Owner } from "./owned"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let backend: MockBackend
let stop: () => void

beforeEach(async () => {
  ;({ backend, stop } = await startTestApp())
})
afterEach(() => stop())

const project = () => useProjectStore.getState().project
const question = () => usePromptStore.getState().confirm
const names = (owner: Owner) =>
  automationOwnedBy(project(), owner).automations.map((item) => item.name)

async function automate(target: AutomationTarget): Promise<number> {
  const result = await backend.automate(target)
  await settle()
  return result.created[0]
}

/** Answers the question that is up, if one is. */
async function answer(choice: string | null) {
  await settle()
  question()?.resolve(choice)
  await settle()
}

/**
 * A project with automation on everything that can own some: the Kick
 * channel, the Kick track, an effect on it, and sends from and to it.
 */
async function automateEverything() {
  const kick = project().channels[0].id
  const [, kickTrack, clapTrack] = project().mixer.tracks.map((t) => t.id)
  const added = await dispatch({
    type: "addEffect",
    track: kickTrack,
    kind: "compressor",
  })
  const effect = added!.created[0]
  await dispatch({ type: "setSend", from: kickTrack, to: clapTrack, gain: 1 })
  const hat = project().mixer.tracks[3].id
  await dispatch({ type: "setSend", from: hat, to: kickTrack, gain: 1 })
  await automate({ type: "channelVolume", channel: kick })
  await automate({ type: "channelPan", channel: kick })
  await automate({ type: "trackVolume", track: kickTrack })
  await automate({ type: "effectMix", track: kickTrack, effect })
  await automate({ type: "effectParam", track: kickTrack, effect, param: 0 })
  await automate({ type: "sendGain", track: kickTrack, target: clapTrack })
  await automate({ type: "sendGain", track: hat, target: kickTrack })
  await automate({ type: "trackPan", track: clapTrack })
  await automate({ type: "tempo" })
  return { kick, kickTrack, clapTrack, hat, effect }
}

describe("what goes with a thing", () => {
  it("finds the automations of a channel, an effect, a send and a track", async () => {
    const ids = await automateEverything()
    expect(names({ type: "channel", channel: ids.kick })).toHaveLength(2)
    expect(names({ type: "effect", effect: ids.effect })).toHaveLength(2)
    expect(
      names({ type: "send", track: ids.kickTrack, target: ids.clapTrack })
    ).toHaveLength(1)
    // A track: its fader, its effect's two, and the sends from and to it.
    expect(names({ type: "track", track: ids.kickTrack })).toHaveLength(5)
    expect(names({ type: "track", track: ids.clapTrack })).toHaveLength(2)
    // The tempo's automation is nobody's.
    const owned = new Set(
      (
        [
          { type: "channel", channel: ids.kick },
          { type: "track", track: ids.kickTrack },
          { type: "track", track: ids.clapTrack },
          { type: "track", track: ids.hat },
        ] as Owner[]
      ).flatMap(names)
    )
    expect(owned.has("Tempo")).toBe(false)
  })

  it("predicts exactly what each deletion takes from the project", async () => {
    const ids = await automateEverything()
    const cases: [Owner, () => Promise<unknown>][] = [
      [
        { type: "channel", channel: ids.kick },
        () => dispatch({ type: "removeChannel", id: ids.kick }),
      ],
      [
        { type: "effect", effect: ids.effect },
        () =>
          dispatch({
            type: "removeEffect",
            track: ids.kickTrack,
            effect: ids.effect,
          }),
      ],
      [
        { type: "effect", effect: ids.effect },
        () =>
          dispatch({
            type: "replaceEffect",
            track: ids.kickTrack,
            effect: ids.effect,
            kind: "delay",
          }),
      ],
      [
        { type: "send", track: ids.kickTrack, target: ids.clapTrack },
        () =>
          dispatch({
            type: "setSend",
            from: ids.kickTrack,
            to: ids.clapTrack,
          }),
      ],
      [
        { type: "track", track: ids.kickTrack },
        () => dispatch({ type: "removeMixerTrack", id: ids.kickTrack }),
      ],
      [
        { type: "track", track: ids.clapTrack },
        () => dispatch({ type: "removeMixerTrack", id: ids.clapTrack }),
      ],
    ]
    for (const [owner, remove] of cases) {
      const before = project().automations.map((item) => item.id)
      const clipsBefore = project().playlist.clips.length
      const owned = automationOwnedBy(project(), owner)
      await remove()
      await settle()
      const after = new Set(project().automations.map((item) => item.id))
      const gone = before.filter((id) => !after.has(id)).sort()
      // The real document, which the mock runs, is what decides.
      expect(gone, owner.type).toEqual(
        owned.automations.map((item) => item.id).sort()
      )
      expect(clipsBefore - project().playlist.clips.length).toBe(owned.clips)
      await undo()
      await settle()
    }
  })

  it("says it in a sentence, and nothing when there is nothing", () => {
    const automation = (name: string) =>
      ({ id: 1, name }) as unknown as Parameters<
        typeof ownedSentence
      >[0]["automations"][number]
    expect(ownedSentence({ automations: [], clips: 0 })).toBeNull()
    expect(
      ownedSentence({ automations: [automation("Kick volume")], clips: 1 })
    ).toBe(
      'The automation "Kick volume" and its clip on the playlist are deleted with it.'
    )
    expect(
      ownedSentence({ automations: [automation("Kick volume")], clips: 3 })
    ).toBe(
      'The automation "Kick volume" and its 3 clips on the playlist are deleted with it.'
    )
    expect(
      ownedSentence({ automations: [automation("Kick volume")], clips: 0 })
    ).toBe('The automation "Kick volume" is deleted with it.')
    expect(
      ownedSentence({
        automations: [automation("A"), automation("B")],
        clips: 5,
      })
    ).toBe("2 automations and their 5 clips on the playlist are deleted with it.")
  })
})

describe("deleting something that owns automation", () => {
  it("removes an effect or a send at once when nothing automates it", async () => {
    const [, kickTrack, clapTrack] = project().mixer.tracks.map((t) => t.id)
    const added = await dispatch({
      type: "addEffect",
      track: kickTrack,
      kind: "compressor",
    })
    await dispatch({ type: "setSend", from: kickTrack, to: clapTrack, gain: 1 })

    const replacing = replaceEffect(added!.created[0], "delay")
    await settle()
    expect(question()).toBeNull()
    await replacing
    const delay = project().mixer.tracks[1].effects[0]
    expect(delay.params.type).toBe("delay")

    const removing = removeEffect(delay.id)
    await settle()
    expect(question()).toBeNull()
    await removing
    expect(project().mixer.tracks[1].effects).toEqual([])

    const unsending = removeSend(kickTrack, clapTrack)
    await settle()
    expect(question()).toBeNull()
    await unsending
    expect(project().mixer.tracks[1].sends).toEqual([])
  })

  it("asks before removing or replacing an automated effect, and says what goes", async () => {
    const ids = await automateEverything()
    const effects = () => project().mixer.tracks[1].effects
    const count = project().automations.length

    const removing = removeEffect(ids.effect)
    await settle()
    expect(question()?.title).toBe("Remove the Compressor?")
    expect(question()?.description).toBe(
      "2 automations and their 2 clips on the playlist are deleted with it. Undo brings them back."
    )
    await answer(null)
    await removing
    expect(effects()).toHaveLength(1)
    expect(project().automations).toHaveLength(count)

    const replacing = replaceEffect(ids.effect, "delay")
    await settle()
    expect(question()?.title).toBe("Replace the Compressor?")
    await answer("go")
    await replacing
    expect(effects()[0].params.type).toBe("delay")
    expect(project().automations).toHaveLength(count - 2)
  })

  it("asks before removing an automated send", async () => {
    const ids = await automateEverything()
    const sends = () => project().mixer.tracks[1].sends
    const removing = removeSend(ids.kickTrack, ids.clapTrack)
    await settle()
    expect(question()?.title).toBe('Remove the send to "Clap"?')
    expect(question()?.description).toContain(
      "and its clip on the playlist are deleted with it."
    )
    await answer(null)
    await removing
    expect(sends()).toHaveLength(1)

    const again = removeSend(ids.kickTrack, ids.clapTrack)
    await answer("remove")
    await again
    expect(sends()).toEqual([])
  })

  it("counts the automation in the questions about a channel and a track", async () => {
    const ids = await automateEverything()
    const channel = deleteChannel(ids.kick)
    await settle()
    expect(question()?.description).toBe(
      "Its steps in every pattern are deleted with it. Its mixer track stays. 2 automations and their 2 clips on the playlist are deleted with it. Undo brings the channel back."
    )
    await answer(null)
    await channel

    const track = deleteTrack(ids.kickTrack)
    await settle()
    expect(question()?.description).toContain(
      "5 automations and their 5 clips on the playlist are deleted with it."
    )
    await answer(null)
    await track

    // A track with nothing but an automated fader still asks.
    await dispatch({ type: "addMixerTrack", name: "Bus" })
    const bus = project().mixer.tracks.at(-1)!.id
    await automate({ type: "trackVolume", track: bus })
    const lone = deleteTrack(bus)
    await settle()
    expect(question()?.description).toBe(
      'The automation "Bus volume" and its clip on the playlist are deleted with it.'
    )
    await answer(null)
    await lone
  })

  it("says nothing more about a channel nothing automates", async () => {
    const channel = deleteChannel(project().channels[1].id)
    await settle()
    expect(question()?.description).toBe(
      "Its steps in every pattern are deleted with it. Its mixer track stays. Undo brings the channel back."
    )
    await answer(null)
    await channel
  })
})
