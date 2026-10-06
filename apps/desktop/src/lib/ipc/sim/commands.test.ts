import { describe, expect, it } from "vitest"

import type { Command, Project } from "@/bindings"
import { MASTER_TRACK, MAX_MIXER_TRACKS, TICKS_PER_STEP } from "@/lib/units"

import { applyCommand, CommandError } from "./commands"
import { demoProject, newProject } from "./project"

/** Applies commands in order and returns the project and the last result. */
function run(project: Project, ...commands: Command[]) {
  let applied = applyCommand(project, commands[0])
  for (const command of commands.slice(1)) {
    applied = applyCommand(applied.project, command)
  }
  return applied
}

function withChannel(name = "Kick") {
  const applied = applyCommand(newProject(), { type: "addChannel", name })
  return { project: applied.project, channel: applied.created[0] }
}

describe("ids and the demo project", () => {
  it("starts with four sampler channels and a beat", () => {
    const project = demoProject()
    expect(project.channels.map((channel) => channel.name)).toEqual([
      "Kick",
      "Clap",
      "Hat",
      "Snare",
    ])
    const kick = project.patterns[0].lanes.find(
      (lane) => lane.channel === project.channels[0].id
    )
    expect(kick?.notes.map((note) => note.start / TICKS_PER_STEP)).toEqual([
      0, 4, 8, 12,
    ])
  })

  it("hands out ids from nextId and never repeats one", () => {
    const project = demoProject()
    const ids = [
      ...project.samples.map((item) => item.id),
      ...project.channels.map((item) => item.id),
      ...project.patterns.map((item) => item.id),
      ...project.mixer.tracks.map((item) => item.id),
      ...project.patterns.flatMap((pattern) =>
        pattern.lanes.flatMap((lane) => lane.notes.map((note) => note.id))
      ),
    ]
    expect(new Set(ids).size).toBe(ids.length)
    expect(Math.max(...ids)).toBeLessThan(project.nextId)
  })
})

describe("channels", () => {
  it("adds a mixer track for a new channel and routes the channel to it", () => {
    const start = newProject()
    const { project, created, touched, label } = applyCommand(start, {
      type: "addChannel",
      name: "Kick",
    })
    const [channelId, trackId] = created
    const channel = project.channels[0]
    const track = project.mixer.tracks[1]

    expect(channel.id).toBe(channelId)
    expect(channel.mixerTrack).toBe(trackId)
    expect(track).toMatchObject({ id: trackId, name: "Kick", output: 0 })
    expect(touched).toMatchObject({ channels: true, mixer: true })
    expect(label).toBe("Add channel")
    expect(start.channels).toHaveLength(0)
  })

  it("uses an existing mixer track when one is given", () => {
    const { project, created } = applyCommand(newProject(), {
      type: "addChannel",
      mixerTrack: MASTER_TRACK,
    })
    expect(created).toHaveLength(1)
    expect(project.mixer.tracks).toHaveLength(1)
    expect(project.channels[0]).toMatchObject({
      name: "Sampler",
      mixerTrack: MASTER_TRACK,
    })
  })

  it("names a channel after its sample and inserts it at an index", () => {
    const { project } = run(
      newProject(),
      { type: "addChannel", name: "First" },
      {
        type: "addSample",
        name: "Rim",
        path: { kind: "factory", path: "Rim.wav" },
      }
    )
    const sample = project.samples[0].id
    const next = applyCommand(project, { type: "addChannel", sample, index: 0 })
    expect(next.project.channels.map((channel) => channel.name)).toEqual([
      "Rim",
      "First",
    ])
  })

  it("removes a channel's notes from every pattern but keeps its track", () => {
    const { project, channel } = withChannel()
    const pattern = project.patterns[0].id
    const { project: after, touched } = run(
      project,
      { type: "toggleStep", pattern, channel, step: 0 },
      { type: "removeChannel", id: channel }
    )
    expect(after.channels).toHaveLength(0)
    expect(after.patterns[0].lanes).toHaveLength(0)
    expect(after.mixer.tracks).toHaveLength(2)
    expect(touched.patterns).toContain(pattern)
  })

  it("duplicates a channel below the original with its notes and track", () => {
    const { project, channel } = withChannel()
    const pattern = project.patterns[0].id
    const { project: after, created } = run(
      project,
      { type: "toggleStep", pattern, channel, step: 3 },
      { type: "duplicateChannel", id: channel }
    )
    const [original, copy] = after.channels
    expect(copy).toMatchObject({
      id: created[0],
      name: "Kick 2",
      mixerTrack: original.mixerTrack,
    })
    const [first, second] = after.patterns[0].lanes
    expect(second.channel).toBe(copy.id)
    expect(second.notes[0].start).toBe(first.notes[0].start)
    expect(second.notes[0].id).not.toBe(first.notes[0].id)
  })

  it("rejects values outside their range and leaves the project alone", () => {
    const { project, channel } = withChannel()
    expect(() =>
      applyCommand(project, {
        type: "updateChannel",
        id: channel,
        patch: { volume: 2.5 },
      })
    ).toThrow(CommandError)
    expect(() =>
      applyCommand(project, {
        type: "updateSampler",
        id: channel,
        patch: { start: 0.8, end: 0.5 },
      })
    ).toThrow("The sample end must come after its start")
    expect(() =>
      applyCommand(project, { type: "removeChannel", id: 999 })
    ).toThrow("channel 999 does not exist")
  })

  it("labels an edit after the one field it changes", () => {
    const { project, channel } = withChannel()
    const label = (patch: { muted?: boolean; volume?: number; pan?: number }) =>
      applyCommand(project, { type: "updateChannel", id: channel, patch }).label
    expect(label({ muted: true })).toBe("Mute channel")
    expect(label({ volume: 0.5 })).toBe("Change channel volume")
    expect(label({ volume: 0.5, pan: 0.1 })).toBe("Change channel")
  })
})

describe("samples", () => {
  const add: Command = {
    type: "addSample",
    name: "Kick",
    path: { kind: "factory", path: "Kick.wav" },
  }

  it("reports the existing id when the path is already in the pool", () => {
    const first = applyCommand(newProject(), add)
    const second = applyCommand(first.project, add)
    expect(second.created).toEqual(first.created)
    expect(second.project.samples).toHaveLength(1)
  })

  it("refuses to remove a sample a channel still uses", () => {
    const { project, created } = applyCommand(newProject(), add)
    const used = applyCommand(project, {
      type: "addChannel",
      sample: created[0],
    }).project
    expect(() =>
      applyCommand(used, { type: "removeSample", id: created[0] })
    ).toThrow('Channel "Kick" still uses this sample')
  })
})

describe("steps and notes", () => {
  it("turns a step on as a one-step note at the default key, then off", () => {
    const { project, channel } = withChannel()
    const pattern = project.patterns[0].id
    const toggle: Command = { type: "toggleStep", pattern, channel, step: 5 }

    const on = applyCommand(project, toggle)
    const note = on.project.patterns[0].lanes[0].notes[0]
    expect(note).toMatchObject({
      id: on.created[0],
      start: 5 * TICKS_PER_STEP,
      length: TICKS_PER_STEP,
      key: 60,
      velocity: 0.8,
    })

    const off = applyCommand(on.project, toggle)
    expect(off.created).toEqual([])
    expect(off.project.patterns[0].lanes).toHaveLength(0)
  })

  it("keeps notes sorted by start, then key, then id", () => {
    const { project, channel } = withChannel()
    const pattern = project.patterns[0].id
    const { project: after, created } = applyCommand(project, {
      type: "addNotes",
      pattern,
      channel,
      notes: [
        { start: 480, length: 240, key: 64 },
        { start: 0, length: 240, key: 67 },
        { start: 0, length: 240, key: 60, velocity: 1 },
      ],
    })
    expect(created).toHaveLength(3)
    expect(
      after.patterns[0].lanes[0].notes.map((note) => [note.start, note.key])
    ).toEqual([
      [0, 60],
      [0, 67],
      [480, 64],
    ])
  })

  it("updates and removes notes, and fails on a note that is not there", () => {
    const { project, channel } = withChannel()
    const pattern = project.patterns[0].id
    const added = applyCommand(project, {
      type: "addNotes",
      pattern,
      channel,
      notes: [{ start: 0, length: 240, key: 60 }],
    })
    const id = added.created[0]
    const moved = applyCommand(added.project, {
      type: "updateNotes",
      pattern,
      channel,
      updates: [{ id, patch: { start: 960, velocity: 0.25 } }],
    })
    expect(moved.project.patterns[0].lanes[0].notes[0]).toMatchObject({
      start: 960,
      velocity: 0.25,
    })
    expect(() =>
      applyCommand(added.project, {
        type: "removeNotes",
        pattern,
        channel,
        notes: [id, 4242],
      })
    ).toThrow("note 4242 does not exist")
    const cleared = applyCommand(added.project, {
      type: "clearLane",
      pattern,
      channel,
    })
    expect(cleared.project.patterns[0].lanes).toHaveLength(0)
  })
})

describe("patterns", () => {
  it("adds patterns with numbered names", () => {
    const { project, created, touched } = applyCommand(newProject(), {
      type: "addPattern",
    })
    expect(project.patterns.map((pattern) => pattern.name)).toEqual([
      "Pattern 1",
      "Pattern 2",
    ])
    expect(touched.patternList).toBe(true)
    expect(touched.patterns).toEqual(created)
  })

  it("will not remove the last pattern", () => {
    const project = newProject()
    expect(() =>
      applyCommand(project, {
        type: "removePattern",
        id: project.patterns[0].id,
      })
    ).toThrow("A project needs at least one pattern")
  })

  it("removes the clips that play a removed pattern", () => {
    const start = newProject()
    const first = start.patterns[0].id
    const { project, created } = run(
      start,
      { type: "addPattern" },
      { type: "addPlaylistTrack" }
    )
    const track = created[0]
    const withClips = applyCommand(project, {
      type: "addClips",
      clips: [
        { track, start: 0, content: { type: "pattern", pattern: first } },
        {
          track,
          start: 3840,
          content: { type: "pattern", pattern: project.patterns[1].id },
        },
      ],
    })
    expect(withClips.project.playlist.clips[0].length).toBe(16 * TICKS_PER_STEP)

    const removed = applyCommand(withClips.project, {
      type: "removePattern",
      id: first,
    })
    expect(removed.project.playlist.clips).toHaveLength(1)
    expect(removed.touched.playlist).toBe(true)
  })

  it("duplicates a pattern right after the original with new note ids", () => {
    const { project, channel } = withChannel()
    const pattern = project.patterns[0].id
    const { project: after, created } = run(
      project,
      { type: "toggleStep", pattern, channel, step: 0 },
      { type: "addPattern" },
      { type: "duplicatePattern", id: pattern }
    )
    expect(after.patterns.map((item) => item.name)).toEqual([
      "Pattern 1",
      "Pattern 3",
      "Pattern 2",
    ])
    expect(after.patterns[1].id).toBe(created[0])
    expect(after.patterns[1].lanes[0].notes[0].id).not.toBe(
      after.patterns[0].lanes[0].notes[0].id
    )
  })
})

describe("mixer", () => {
  it("will not remove the master", () => {
    expect(() =>
      applyCommand(newProject(), { type: "removeMixerTrack", id: MASTER_TRACK })
    ).toThrow("The master track cannot be removed")
  })

  it("reroutes to the master what a removed track was feeding", () => {
    const { project, created } = run(
      newProject(),
      { type: "addChannel", name: "Kick" },
      { type: "addMixerTrack", name: "Bus" }
    )
    const bus = created[0]
    const channel = project.channels[0]
    const routed = run(
      project,
      { type: "setTrackOutput", id: channel.mixerTrack, output: bus },
      { type: "updateChannel", id: channel.id, patch: { mixerTrack: bus } },
      { type: "setSend", from: channel.mixerTrack, to: bus, gain: 1 }
    ).project

    const { project: after, touched } = applyCommand(routed, {
      type: "removeMixerTrack",
      id: bus,
    })
    expect(after.channels[0].mixerTrack).toBe(MASTER_TRACK)
    expect(after.mixer.tracks[1]).toMatchObject({
      output: MASTER_TRACK,
      sends: [],
    })
    expect(touched).toMatchObject({ channels: true, mixer: true })
  })

  it("refuses routing that would form a cycle", () => {
    const { project, created } = run(
      newProject(),
      { type: "addMixerTrack", name: "A" },
      { type: "addMixerTrack", name: "B" }
    )
    const b = created[0]
    const a = project.mixer.tracks[1].id
    const chained = applyCommand(project, {
      type: "setTrackOutput",
      id: a,
      output: b,
    }).project

    expect(() =>
      applyCommand(chained, { type: "setTrackOutput", id: b, output: a })
    ).toThrow("feed the track back into itself")
    expect(() =>
      applyCommand(chained, { type: "setSend", from: b, to: a, gain: 0.5 })
    ).toThrow("feed the track back into itself")
  })

  it("adds, changes and removes a send", () => {
    const { project, created } = run(
      newProject(),
      { type: "addMixerTrack" },
      { type: "addMixerTrack" }
    )
    const to = created[0]
    const from = project.mixer.tracks[1].id
    const added = applyCommand(project, {
      type: "setSend",
      from,
      to,
      gain: 0.5,
    })
    expect(added.label).toBe("Add send")
    const changed = applyCommand(added.project, {
      type: "setSend",
      from,
      to,
      gain: 1,
    })
    expect(changed.project.mixer.tracks[1].sends).toEqual([
      { target: to, gain: 1 },
    ])
    const removed = applyCommand(changed.project, { type: "setSend", from, to })
    expect(removed.project.mixer.tracks[1].sends).toEqual([])
  })

  it("stops at the mixer's track limit", () => {
    let project = newProject()
    for (let count = 1; count < MAX_MIXER_TRACKS; count += 1) {
      project = applyCommand(project, { type: "addMixerTrack" }).project
    }
    expect(() => applyCommand(project, { type: "addMixerTrack" })).toThrow(
      "The mixer is full"
    )
    expect(() => applyCommand(project, { type: "addChannel" })).toThrow(
      "The mixer is full"
    )
  })
})

describe("settings and batches", () => {
  it("validates tempo and time signature", () => {
    const project = newProject()
    expect(() =>
      applyCommand(project, { type: "updateSettings", patch: { tempoBpm: 5 } })
    ).toThrow("Tempo must be between 10 and 522")
    expect(() =>
      applyCommand(project, {
        type: "updateSettings",
        patch: { timeSignature: { numerator: 4, denominator: 3 } },
      })
    ).toThrow("The beat unit must be 2, 4, 8 or 16")
    const changed = applyCommand(project, {
      type: "updateSettings",
      patch: { tempoBpm: 174 },
    })
    expect(changed.project.settings.tempoBpm).toBe(174)
    expect(changed.label).toBe("Change tempo")
  })

  it("applies a batch as a whole and collects what it created", () => {
    const { project, created, label } = applyCommand(newProject(), {
      type: "batch",
      commands: [{ type: "addPattern" }, { type: "addMixerTrack" }],
    })
    expect(created).toHaveLength(2)
    expect(project.patterns).toHaveLength(2)
    expect(label).toBe("Add pattern")
  })

  it("applies nothing from a batch when one command fails", () => {
    const project = newProject()
    expect(() =>
      applyCommand(project, {
        type: "batch",
        label: "Set up",
        commands: [{ type: "addPattern" }, { type: "removeChannel", id: 77 }],
      })
    ).toThrow("channel 77 does not exist")
    expect(project.patterns).toHaveLength(1)
  })
})
