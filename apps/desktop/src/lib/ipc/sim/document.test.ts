import { describe, expect, it } from "vitest"

// The source of the generated binding, to read the list of commands from.
import commandBinding from "@/bindings/Command.ts?raw"
import type {
  Command,
  EffectParams,
  InstrumentParams,
  Project,
} from "@/bindings"
import descriptors from "@/bindings/descriptors.json"

import { SimDocument } from "./document"
import { buildProject, demoProject, emptyProject } from "./project"

/** A project with something of every kind in it, and the ids of the parts. */
function fullProject() {
  const ids = {
    sample: 0,
    spareSample: 0,
    channel: 0,
    synth: 0,
    effect: 0,
    sparePattern: 0,
    track: 0,
    playlistTrack: 0,
    lowerTrack: 0,
    clip: 0,
    audioClip: 0,
    automation: 0,
  }
  const base = demoProject()
  const project = buildProject(base, (run) => {
    ids.sample = base.samples[0].id
    ids.channel = base.channels[0].id
    ;[ids.spareSample] = run({
      type: "addSample",
      name: "Spare",
      path: { kind: "factory", path: "Drums/Kicks/Kick 02.wav" },
    })
    ;[ids.sparePattern] = run({ type: "addPattern" })
    ;[ids.track] = run({ type: "addMixerTrack" })
    ;[ids.synth] = run({ type: "addChannel", instrument: "subtractiveSynth" })
    ;[ids.effect] = run({
      type: "addEffect",
      track: ids.track,
      kind: "compressor",
    })
    run({ type: "addEffect", track: ids.track, kind: "delay" })
    ;[ids.playlistTrack] = run({ type: "addPlaylistTrack" })
    ;[ids.clip] = run({
      type: "addClips",
      clips: [
        {
          track: ids.playlistTrack,
          start: 0,
          content: { type: "pattern", pattern: base.patterns[0].id },
        },
      ],
    })
    ;[ids.lowerTrack] = run({ type: "addPlaylistTrack" })
    ;[ids.audioClip] = run({
      type: "addClips",
      clips: [
        {
          track: ids.lowerTrack,
          start: 0,
          length: 1920,
          content: {
            type: "audio",
            sample: ids.sample,
            mixerTrack: ids.track,
            gain: 1,
            pan: 0,
            fadeIn: 0,
            fadeOut: 0,
            reverse: false,
            pitch: 0,
          },
        },
      ],
    })
    ;[ids.automation] = run({
      type: "addAutomation",
      target: { type: "trackVolume", track: ids.track },
    })
    run({
      type: "addClips",
      clips: [
        {
          track: ids.lowerTrack,
          start: 3840,
          content: { type: "automation", automation: ids.automation },
        },
      ],
    })
  })
  const scratch = SimDocument.create(project)
  const [pluginChannel] = scratch.dispatch({
    type: "addPluginInstrument",
    plugin: pluginBinding(),
  }).created
  const withPlugin = scratch.project()
  scratch.dispose()
  const pattern = withPlugin.patterns[0]
  const note = pattern.lanes.find((lane) => lane.channel === ids.channel)!
    .notes[0].id
  return {
    project: withPlugin,
    ids: { ...ids, pattern: pattern.id, note, pluginChannel },
  }
}

function pluginBinding() {
  return {
    target: { type: "instrument" as const, channel: 0 },
    format: "clap",
    path: "missing/test.clap",
    id: "test.synth",
    name: "Test plugin",
    state: [1],
    parameters: [
      {
        id: 7,
        name: "Level",
        min: 0,
        max: 1,
        value: 0.5,
        stepped: false,
        readOnly: false,
        automatable: true,
      },
    ],
  }
}

type Ids = ReturnType<typeof fullProject>["ids"]

/**
 * One small valid command of every type. The type makes the compiler ask
 * for a new entry when the Rust `Command` gains a variant.
 */
const EVERY_COMMAND: { [Type in Command["type"]]: (ids: Ids) => Command } = {
  addPluginInstrument: () => ({
    type: "addPluginInstrument",
    plugin: pluginBinding(),
  }),
  addPluginEffect: (ids) => ({
    type: "addPluginEffect",
    track: ids.track,
    plugin: pluginBinding(),
  }),
  setPluginParam: (ids) => ({
    type: "setPluginParam",
    target: { type: "instrument", channel: ids.pluginChannel },
    id: 7,
    value: 0.75,
  }),
  setPluginState: (ids) => ({
    type: "setPluginState",
    target: { type: "instrument", channel: ids.pluginChannel },
    state: [1, 2],
  }),
  updateSettings: () => ({ type: "updateSettings", patch: { swing: 0.25 } }),
  addSample: () => ({
    type: "addSample",
    name: "Rim",
    path: { kind: "factory", path: "Drums/Percussion/Rim 01.wav" },
  }),
  removeSample: (ids) => ({ type: "removeSample", id: ids.spareSample }),
  addChannel: (ids) => ({ type: "addChannel", sample: ids.sample, index: 0 }),
  removeChannel: (ids) => ({ type: "removeChannel", id: ids.channel }),
  duplicateChannel: (ids) => ({ type: "duplicateChannel", id: ids.channel }),
  moveChannel: (ids) => ({ type: "moveChannel", id: ids.channel, index: 2 }),
  updateChannel: (ids) => ({
    type: "updateChannel",
    id: ids.channel,
    patch: { name: "Boom", color: 0x112233, mixerTrack: ids.track },
  }),
  setChannelSample: (ids) => ({ type: "setChannelSample", id: ids.channel }),
  updateSampler: (ids) => ({
    type: "updateSampler",
    id: ids.channel,
    patch: { rootKey: 48, start: 0.1, end: 0.9, reverse: true, cutGroup: 3 },
  }),
  setSamplerEnvelope: (ids) => ({
    type: "setSamplerEnvelope",
    id: ids.channel,
    envelope: { attackMs: 5, decayMs: 100, sustain: 0.5, releaseMs: 80 },
  }),
  setInstrumentParam: (ids) => ({
    type: "setInstrumentParam",
    channel: ids.synth,
    param: 1,
    value: 0.5,
  }),
  setInstrumentParams: (ids) => ({
    type: "setInstrumentParams",
    channel: ids.synth,
    params: {
      ...descriptors.instruments.subtractiveSynth.defaults,
      glideMs: 40,
    } as InstrumentParams,
  }),
  addPattern: () => ({ type: "addPattern" }),
  removePattern: (ids) => ({ type: "removePattern", id: ids.sparePattern }),
  duplicatePattern: (ids) => ({ type: "duplicatePattern", id: ids.pattern }),
  movePattern: (ids) => ({ type: "movePattern", id: ids.pattern, index: 1 }),
  updatePattern: (ids) => ({
    type: "updatePattern",
    id: ids.pattern,
    patch: { lengthSteps: 32 },
  }),
  toggleStep: (ids) => ({
    type: "toggleStep",
    pattern: ids.pattern,
    channel: ids.channel,
    step: 3,
  }),
  addNotes: (ids) => ({
    type: "addNotes",
    pattern: ids.pattern,
    channel: ids.channel,
    notes: [{ start: 100, length: 50, key: 64, velocity: 0.5 }],
  }),
  removeNotes: (ids) => ({
    type: "removeNotes",
    pattern: ids.pattern,
    channel: ids.channel,
    notes: [ids.note],
  }),
  updateNotes: (ids) => ({
    type: "updateNotes",
    pattern: ids.pattern,
    channel: ids.channel,
    updates: [{ id: ids.note, patch: { key: 72, pan: -0.5 } }],
  }),
  clearLane: (ids) => ({
    type: "clearLane",
    pattern: ids.pattern,
    channel: ids.channel,
  }),
  addMixerTrack: () => ({ type: "addMixerTrack", name: "Bus" }),
  removeMixerTrack: (ids) => ({ type: "removeMixerTrack", id: ids.track }),
  updateMixerTrack: (ids) => ({
    type: "updateMixerTrack",
    id: ids.track,
    patch: { volume: 0.5, solo: true },
  }),
  setTrackOutput: (ids) => ({ type: "setTrackOutput", id: ids.track }),
  setSend: (ids) => ({ type: "setSend", from: ids.track, to: 0, gain: 0.5 }),
  addEffect: (ids) => ({ type: "addEffect", track: ids.track, kind: "reverb" }),
  removeEffect: (ids) => ({
    type: "removeEffect",
    track: ids.track,
    effect: ids.effect,
  }),
  moveEffect: (ids) => ({
    type: "moveEffect",
    track: ids.track,
    effect: ids.effect,
    index: 1,
  }),
  updateEffect: (ids) => ({
    type: "updateEffect",
    track: ids.track,
    effect: ids.effect,
    patch: { enabled: false, mix: 0.5 },
  }),
  setEffectParam: (ids) => ({
    type: "setEffectParam",
    track: ids.track,
    effect: ids.effect,
    param: 0,
    value: -30,
  }),
  setEffectParams: (ids) => ({
    type: "setEffectParams",
    track: ids.track,
    effect: ids.effect,
    params: {
      ...descriptors.effects.compressor.defaults,
      ratio: 8,
    } as EffectParams,
  }),
  duplicateEffect: (ids) => ({
    type: "duplicateEffect",
    track: ids.track,
    effect: ids.effect,
  }),
  replaceEffect: (ids) => ({
    type: "replaceEffect",
    track: ids.track,
    effect: ids.effect,
    kind: "reverb",
  }),
  addPlaylistTrack: () => ({ type: "addPlaylistTrack", index: 0 }),
  movePlaylistTrack: (ids) => ({
    type: "movePlaylistTrack",
    id: ids.playlistTrack,
    index: 1,
  }),
  removePlaylistTrack: (ids) => ({
    type: "removePlaylistTrack",
    id: ids.playlistTrack,
  }),
  updatePlaylistTrack: (ids) => ({
    type: "updatePlaylistTrack",
    id: ids.playlistTrack,
    patch: { muted: true },
  }),
  addClips: (ids) => ({
    type: "addClips",
    clips: [
      {
        track: ids.playlistTrack,
        start: 3840,
        length: 960,
        content: { type: "pattern", pattern: ids.pattern },
      },
      {
        track: ids.playlistTrack,
        start: 7680,
        length: 480,
        offset: 120,
        muted: true,
        content: {
          type: "audio",
          sample: ids.sample,
          mixerTrack: 0,
          gain: 0.5,
          pan: -0.25,
          fadeIn: 60,
          fadeOut: 120,
          reverse: true,
          pitch: 7,
        },
      },
      {
        track: ids.playlistTrack,
        start: 9600,
        content: { type: "automation", automation: ids.automation },
      },
    ],
  }),
  removeClips: (ids) => ({ type: "removeClips", clips: [ids.clip] }),
  updateClips: (ids) => ({
    type: "updateClips",
    updates: [{ id: ids.clip, patch: { start: 960, offset: 240 } }],
  }),
  updateAudioClips: (ids) => ({
    type: "updateAudioClips",
    updates: [
      {
        id: ids.audioClip,
        patch: { gain: 0.5, pitch: -12, reverse: true, fadeIn: 240 },
      },
    ],
  }),
  addAutomation: (ids) => ({
    type: "addAutomation",
    target: { type: "trackPan", track: ids.track },
  }),
  removeAutomation: (ids) => ({
    type: "removeAutomation",
    id: ids.automation,
  }),
  updateAutomation: (ids) => ({
    type: "updateAutomation",
    id: ids.automation,
    patch: { name: "Ride", color: 0x334455 },
  }),
  setAutomationPoints: (ids) => ({
    type: "setAutomationPoints",
    id: ids.automation,
    points: [
      { tick: 0, value: 0, curve: 0.5, hold: false },
      { tick: 3840, value: 1, curve: 0, hold: true },
    ],
  }),
  duplicateAutomation: (ids) => ({
    type: "duplicateAutomation",
    id: ids.automation,
  }),
  batch: (ids) => ({
    type: "batch",
    label: "Two things",
    commands: [
      { type: "addPattern" },
      { type: "moveChannel", id: ids.channel, index: 1 },
    ],
  }),
}

/** A project as the document reports it, apart from the id counter. */
const content = (project: Project) => ({ ...project, nextId: 0 })

describe("every command of the binding", () => {
  it("is covered: the binding and the list name the same commands", () => {
    const inBinding = [...commandBinding.matchAll(/"type": "(\w+)"/g)].map(
      (match) => match[1]
    )
    expect(inBinding.length).toBeGreaterThan(30)
    expect(Object.keys(EVERY_COMMAND).sort()).toEqual([...inBinding].sort())
  })

  it.each(Object.keys(EVERY_COMMAND) as Command["type"][])(
    "%s is applied by the WebAssembly document, undone and redone",
    (type) => {
      const { project, ids } = fullProject()
      const document = SimDocument.create(project)
      const before = document.snapshot(null).project

      const result = document.dispatch(EVERY_COMMAND[type](ids))
      const after = document.snapshot(null).project
      expect(content(after)).not.toEqual(content(before))
      expect(result.patch.history.entries).toHaveLength(1)
      // The copy kept from patches is the document's project exactly.
      expect(document.project()).toEqual(after)

      document.undo()
      expect(content(document.project())).toEqual(content(before))
      expect(document.isDirty()).toBe(false)
      document.redo()
      expect(document.project()).toEqual(after)
      expect(document.project()).toEqual(document.snapshot(null).project)
      document.dispose()
    }
  )
})

describe("SimDocument", () => {
  it("follows a history jump across added, moved and removed patterns", () => {
    const document = SimDocument.create(demoProject())
    const first = document.project().patterns[0].id
    const [second] = document.dispatch({ type: "addPattern" }).created
    document.dispatch({ type: "movePattern", id: second, index: 0 })
    document.dispatch({ type: "removePattern", id: first })
    document.dispatch({ type: "addChannel" })
    expect(document.project().patterns.map((pattern) => pattern.id)).toEqual([
      second,
    ])

    for (const cursor of [1, 4, 2, 0, 3]) {
      const patch = document.jump(cursor)
      expect(patch.history.cursor).toBe(cursor)
      expect(document.project()).toEqual(document.snapshot(null).project)
    }
    expect(document.isDirty()).toBe(true)
    document.dispose()
  })

  it("knows the id counter and the dirty flag without asking twice", () => {
    const document = SimDocument.create(emptyProject())
    expect(document.project().nextId).toBe(2)
    expect(document.isDirty()).toBe(false)

    document.dispatch({ type: "addChannel" })
    expect(document.project().nextId).toBe(4)
    expect(document.isDirty()).toBe(true)

    // Undo takes the channel away, but never hands its ids out again.
    document.undo()
    expect(document.project().nextId).toBe(4)
    expect(document.isDirty()).toBe(false)
    expect(document.dispatch({ type: "addPattern" }).created).toEqual([4])

    expect(document.markSaved()).toMatchObject({ dirty: false })
    expect(document.isDirty()).toBe(false)
    document.dispose()
  })

  it("opens the file it wrote, and nothing that is not a project", () => {
    const document = SimDocument.create(demoProject())
    document.dispatch({ type: "addPattern", name: "Fill" })
    const reopened = SimDocument.open(document.fileText())
    expect(reopened.project()).toEqual(document.project())
    expect(reopened.isDirty()).toBe(false)
    expect(() => SimDocument.open("[]")).toThrow(
      "this is not a Windfall project"
    )
    document.dispose()
    reopened.dispose()
  })

  it("is closed for good by dispose", () => {
    const document = SimDocument.create(emptyProject())
    document.dispose()
    document.dispose()
    expect(() => document.undo()).toThrow("This document was closed.")
    expect(() => document.snapshot(null)).toThrow("This document was closed.")
  })
})
