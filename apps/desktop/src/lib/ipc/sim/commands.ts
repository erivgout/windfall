import type {
  Channel,
  ChannelId,
  Clip,
  Command,
  Lane,
  MixerTrack,
  Note,
  NotePatch,
  Pattern,
  PatternId,
  Project,
  TrackId,
} from "@/bindings"
import {
  DEFAULT_CHANNEL_VOLUME,
  DEFAULT_KEY,
  DEFAULT_PATTERN_STEPS,
  DEFAULT_VELOCITY,
  MASTER_TRACK,
  MAX_GAIN,
  MAX_MIXER_TRACKS,
  MAX_PATTERN_STEPS,
  MAX_TEMPO_BPM,
  MIN_TEMPO_BPM,
  TICKS_PER_STEP,
} from "@/lib/units"

import { emptyTouched, touchPattern, type Touched } from "./touched"

/** Why a command could not be applied. The project is unchanged. */
export class CommandError extends Error {
  override name = "CommandError"
}

export type Applied = {
  project: Project
  /** Ids the command created, in the order its documentation gives. */
  created: number[]
  touched: Touched
  label: string
}

/** Colors new channels, patterns and mixer tracks cycle through. */
export const ITEM_COLORS = [
  0xe5488f, 0xf08a3c, 0xe6c13d, 0x58c26b, 0x38bdb1, 0x4f9cf0, 0x8b7cf6,
  0xc86be0,
]

type Draft = { project: Project; touched: Touched; created: number[] }

/**
 * Applies a command to a project and returns the new project. The input is
 * never changed, so a command that throws leaves nothing half done.
 */
export function applyCommand(project: Project, command: Command): Applied {
  const draft: Draft = { project, touched: emptyTouched(), created: [] }
  const label = run(draft, command)
  return {
    project: draft.project,
    created: draft.created,
    touched: draft.touched,
    label,
  }
}

function invalid(reason: string): never {
  throw new CommandError(reason)
}

function notFound(kind: string, id: number): never {
  throw new CommandError(`${kind} ${id} does not exist`)
}

function requireRange(value: number, min: number, max: number, what: string) {
  if (!Number.isFinite(value) || value < min || value > max) {
    invalid(`${what} must be between ${min} and ${max}`)
  }
}

function requireInteger(value: number, min: number, max: number, what: string) {
  if (!Number.isInteger(value)) invalid(`${what} must be a whole number`)
  requireRange(value, min, max, what)
}

function alloc(draft: Draft): number {
  const id = draft.project.nextId
  draft.project = { ...draft.project, nextId: id + 1 }
  return id
}

function colorFor(count: number): number {
  return ITEM_COLORS[count % ITEM_COLORS.length]
}

function numberedName(prefix: string, start: number, taken: string[]): string {
  let n = start
  while (taken.includes(`${prefix} ${n}`)) n += 1
  return `${prefix} ${n}`
}

/** "Kick" becomes "Kick 2", "Kick 2" becomes "Kick 3". */
function copyName(name: string, taken: string[]): string {
  const base = name.replace(/ \d+$/, "")
  return numberedName(base, 2, taken)
}

function moved<T>(items: T[], from: number, to: number): T[] {
  const next = [...items]
  const [item] = next.splice(from, 1)
  next.splice(to, 0, item)
  return next
}

function channelIndex(project: Project, id: ChannelId): number {
  const index = project.channels.findIndex((channel) => channel.id === id)
  return index < 0 ? notFound("channel", id) : index
}

function patternIndex(project: Project, id: PatternId): number {
  const index = project.patterns.findIndex((pattern) => pattern.id === id)
  return index < 0 ? notFound("pattern", id) : index
}

function trackIndex(project: Project, id: TrackId): number {
  const index = project.mixer.tracks.findIndex((track) => track.id === id)
  return index < 0 ? notFound("mixer track", id) : index
}

function requireSample(project: Project, id: number) {
  if (!project.samples.some((sample) => sample.id === id)) {
    notFound("sample", id)
  }
}

function replaceAt<T>(items: T[], index: number, item: T): T[] {
  const next = [...items]
  next[index] = item
  return next
}

function setChannel(draft: Draft, index: number, channel: Channel) {
  draft.project = {
    ...draft.project,
    channels: replaceAt(draft.project.channels, index, channel),
  }
  draft.touched.channels = true
}

function setPattern(draft: Draft, index: number, pattern: Pattern) {
  draft.project = {
    ...draft.project,
    patterns: replaceAt(draft.project.patterns, index, pattern),
  }
  touchPattern(draft.touched, pattern.id)
}

function setTracks(draft: Draft, tracks: MixerTrack[]) {
  draft.project = { ...draft.project, mixer: { tracks } }
  draft.touched.mixer = true
}

function sortNotes(notes: Note[]): Note[] {
  return [...notes].sort(
    (a, b) => a.start - b.start || a.key - b.key || a.id - b.id
  )
}

function sortClips(clips: Clip[]): Clip[] {
  return [...clips].sort((a, b) => a.start - b.start || a.id - b.id)
}

/** Replaces one channel's notes in a pattern. An empty lane is dropped. */
function withLane(
  pattern: Pattern,
  channel: ChannelId,
  edit: (notes: Note[]) => Note[]
): Pattern {
  const existing = pattern.lanes.find((lane) => lane.channel === channel)
  const notes = sortNotes(edit(existing?.notes ?? []))
  const others = pattern.lanes.filter((lane) => lane.channel !== channel)
  if (notes.length === 0) return { ...pattern, lanes: others }
  const lane: Lane = { channel, notes }
  if (!existing) return { ...pattern, lanes: [...pattern.lanes, lane] }
  return {
    ...pattern,
    lanes: pattern.lanes.map((item) =>
      item.channel === channel ? lane : item
    ),
  }
}

function validateNote(note: Omit<Note, "id">) {
  requireInteger(note.start, 0, 0xffffffff, "Note start")
  requireInteger(note.length, 1, 0xffffffff, "Note length")
  requireInteger(note.key, 0, 127, "Note key")
  requireRange(note.velocity, 0, 1, "Note velocity")
  requireRange(note.pan, -1, 1, "Note pan")
}

function patchNote(note: Note, patch: NotePatch): Note {
  const next: Note = {
    id: note.id,
    start: patch.start ?? note.start,
    length: patch.length ?? note.length,
    key: patch.key ?? note.key,
    velocity: patch.velocity ?? note.velocity,
    pan: patch.pan ?? note.pan,
  }
  validateNote(next)
  return next
}

/** True when audio leaving `from` can arrive at `to` through outputs and sends. */
function reaches(tracks: MixerTrack[], from: TrackId, to: TrackId): boolean {
  const seen = new Set<TrackId>()
  const queue = [from]
  while (queue.length > 0) {
    const id = queue.pop()
    if (id === undefined || seen.has(id)) continue
    if (id === to) return true
    seen.add(id)
    const track = tracks.find((item) => item.id === id)
    if (!track) continue
    if (track.output !== null) queue.push(track.output)
    for (const send of track.sends) queue.push(send.target)
  }
  return false
}

function addMixerTrack(draft: Draft, name: string): TrackId {
  const tracks = draft.project.mixer.tracks
  if (tracks.length >= MAX_MIXER_TRACKS) {
    invalid(`The mixer is full (${MAX_MIXER_TRACKS} tracks)`)
  }
  const id = alloc(draft)
  setTracks(draft, [
    ...tracks,
    {
      id,
      name,
      color: colorFor(tracks.length - 1),
      volume: 1,
      pan: 0,
      muted: false,
      solo: false,
      output: MASTER_TRACK,
      sends: [],
    },
  ])
  return id
}

type Labels<P> = {
  [K in keyof P]?: string | ((value: NonNullable<P[K]>) => string)
}

/** Names an edit after the one field it changes, or falls back. */
function patchLabel<P extends object>(
  patch: P,
  labels: Labels<P>,
  fallback: string
): string {
  const keys = (Object.keys(patch) as (keyof P)[]).filter(
    (key) => patch[key] !== undefined
  )
  if (keys.length !== 1) return fallback
  const key = keys[0]
  const label = labels[key]
  const value = patch[key]
  if (label === undefined || value === undefined || value === null) {
    return fallback
  }
  return typeof label === "function" ? label(value) : label
}

function run(draft: Draft, command: Command): string {
  const { project } = draft
  switch (command.type) {
    case "updateSettings": {
      const { patch } = command
      if (patch.tempoBpm !== undefined) {
        requireRange(patch.tempoBpm, MIN_TEMPO_BPM, MAX_TEMPO_BPM, "Tempo")
      }
      if (patch.timeSignature !== undefined) {
        requireInteger(patch.timeSignature.numerator, 1, 16, "Beats per bar")
        if (![2, 4, 8, 16].includes(patch.timeSignature.denominator)) {
          invalid("The beat unit must be 2, 4, 8 or 16")
        }
      }
      if (patch.swing !== undefined) requireRange(patch.swing, 0, 1, "Swing")
      draft.project = {
        ...project,
        settings: {
          name: patch.name ?? project.settings.name,
          tempoBpm: patch.tempoBpm ?? project.settings.tempoBpm,
          timeSignature: patch.timeSignature ?? project.settings.timeSignature,
          swing: patch.swing ?? project.settings.swing,
        },
      }
      draft.touched.settings = true
      return patchLabel(
        patch,
        {
          name: "Rename project",
          tempoBpm: "Change tempo",
          timeSignature: "Change time signature",
          swing: "Change swing",
        },
        "Change project settings"
      )
    }

    case "addSample": {
      const same = project.samples.find(
        (sample) =>
          sample.path.kind === command.path.kind &&
          sample.path.path === command.path.path
      )
      if (same) {
        draft.created.push(same.id)
        return "Add sample"
      }
      const id = alloc(draft)
      draft.project = {
        ...draft.project,
        samples: [
          ...project.samples,
          { id, name: command.name, path: command.path },
        ],
      }
      draft.touched.samples = true
      draft.created.push(id)
      return "Add sample"
    }

    case "removeSample": {
      requireSample(project, command.id)
      const user = project.channels.find(
        (channel) => channel.source.sample === command.id
      )
      if (user) invalid(`Channel "${user.name}" still uses this sample`)
      draft.project = {
        ...project,
        samples: project.samples.filter((sample) => sample.id !== command.id),
      }
      draft.touched.samples = true
      return "Remove sample"
    }

    case "addChannel": {
      const sample =
        command.sample === undefined
          ? undefined
          : project.samples.find((item) => item.id === command.sample)
      if (command.sample !== undefined && !sample) {
        notFound("sample", command.sample)
      }
      const index = command.index ?? project.channels.length
      requireInteger(index, 0, project.channels.length, "Channel position")
      if (command.mixerTrack !== undefined) {
        trackIndex(project, command.mixerTrack)
      } else if (project.mixer.tracks.length >= MAX_MIXER_TRACKS) {
        invalid(`The mixer is full (${MAX_MIXER_TRACKS} tracks)`)
      }
      const name = command.name ?? sample?.name ?? "Sampler"
      const id = alloc(draft)
      draft.created.push(id)
      let mixerTrack = command.mixerTrack
      if (mixerTrack === undefined) {
        mixerTrack = addMixerTrack(draft, name)
        draft.created.push(mixerTrack)
      }
      const channel: Channel = {
        id,
        name,
        color: colorFor(project.channels.length),
        volume: DEFAULT_CHANNEL_VOLUME,
        pan: 0,
        muted: false,
        solo: false,
        mixerTrack,
        source: {
          type: "sampler",
          sample: command.sample ?? null,
          rootKey: DEFAULT_KEY,
          tune: 0,
          gain: 1,
          start: 0,
          end: 1,
          reverse: false,
          envelope: null,
          cutSelf: false,
          cutGroup: 0,
        },
      }
      const channels = [...project.channels]
      channels.splice(index, 0, channel)
      draft.project = { ...draft.project, channels }
      draft.touched.channels = true
      return "Add channel"
    }

    case "removeChannel": {
      channelIndex(project, command.id)
      draft.project = {
        ...project,
        channels: project.channels.filter(
          (channel) => channel.id !== command.id
        ),
        patterns: project.patterns.map((pattern) => {
          if (!pattern.lanes.some((lane) => lane.channel === command.id)) {
            return pattern
          }
          touchPattern(draft.touched, pattern.id)
          return {
            ...pattern,
            lanes: pattern.lanes.filter((lane) => lane.channel !== command.id),
          }
        }),
      }
      draft.touched.channels = true
      return "Remove channel"
    }

    case "duplicateChannel": {
      const index = channelIndex(project, command.id)
      const original = project.channels[index]
      const id = alloc(draft)
      draft.created.push(id)
      const copy: Channel = {
        ...original,
        id,
        name: copyName(
          original.name,
          project.channels.map((channel) => channel.name)
        ),
        solo: false,
      }
      const channels = [...project.channels]
      channels.splice(index + 1, 0, copy)
      const patterns = project.patterns.map((pattern) => {
        const lane = pattern.lanes.find((item) => item.channel === command.id)
        if (!lane) return pattern
        touchPattern(draft.touched, pattern.id)
        const notes = lane.notes.map((note) => ({ ...note, id: alloc(draft) }))
        return { ...pattern, lanes: [...pattern.lanes, { channel: id, notes }] }
      })
      draft.project = { ...draft.project, channels, patterns }
      draft.touched.channels = true
      return "Duplicate channel"
    }

    case "moveChannel": {
      const index = channelIndex(project, command.id)
      requireInteger(
        command.index,
        0,
        project.channels.length - 1,
        "Channel position"
      )
      draft.project = {
        ...project,
        channels: moved(project.channels, index, command.index),
      }
      draft.touched.channels = true
      return "Move channel"
    }

    case "updateChannel": {
      const index = channelIndex(project, command.id)
      const { patch } = command
      if (patch.volume !== undefined) {
        requireRange(patch.volume, 0, MAX_GAIN, "Channel volume")
      }
      if (patch.pan !== undefined) requireRange(patch.pan, -1, 1, "Pan")
      if (patch.color !== undefined) {
        requireInteger(patch.color, 0, 0xffffff, "Color")
      }
      if (patch.mixerTrack !== undefined) trackIndex(project, patch.mixerTrack)
      const channel = project.channels[index]
      setChannel(draft, index, {
        ...channel,
        name: patch.name ?? channel.name,
        color: patch.color ?? channel.color,
        volume: patch.volume ?? channel.volume,
        pan: patch.pan ?? channel.pan,
        muted: patch.muted ?? channel.muted,
        solo: patch.solo ?? channel.solo,
        mixerTrack: patch.mixerTrack ?? channel.mixerTrack,
      })
      return patchLabel(
        patch,
        {
          name: "Rename channel",
          color: "Change channel color",
          volume: "Change channel volume",
          pan: "Change channel pan",
          muted: (muted) => (muted ? "Mute channel" : "Unmute channel"),
          solo: (solo) => (solo ? "Solo channel" : "Unsolo channel"),
          mixerTrack: "Route channel",
        },
        "Change channel"
      )
    }

    case "setChannelSample": {
      const index = channelIndex(project, command.id)
      if (command.sample !== undefined) requireSample(project, command.sample)
      const channel = project.channels[index]
      setChannel(draft, index, {
        ...channel,
        source: { ...channel.source, sample: command.sample ?? null },
      })
      return command.sample === undefined
        ? "Clear channel sample"
        : "Change channel sample"
    }

    case "updateSampler": {
      const index = channelIndex(project, command.id)
      const { patch } = command
      const channel = project.channels[index]
      const source = {
        ...channel.source,
        rootKey: patch.rootKey ?? channel.source.rootKey,
        tune: patch.tune ?? channel.source.tune,
        gain: patch.gain ?? channel.source.gain,
        start: patch.start ?? channel.source.start,
        end: patch.end ?? channel.source.end,
        reverse: patch.reverse ?? channel.source.reverse,
        cutSelf: patch.cutSelf ?? channel.source.cutSelf,
        cutGroup: patch.cutGroup ?? channel.source.cutGroup,
      }
      requireInteger(source.rootKey, 0, 127, "Root key")
      requireRange(source.tune, -48, 48, "Tune")
      requireRange(source.gain, 0, MAX_GAIN, "Sample gain")
      requireRange(source.start, 0, 1, "Sample start")
      requireRange(source.end, 0, 1, "Sample end")
      if (source.end <= source.start) {
        invalid("The sample end must come after its start")
      }
      requireInteger(source.cutGroup, 0, 255, "Cut group")
      setChannel(draft, index, { ...channel, source })
      return patchLabel(
        patch,
        {
          rootKey: "Change root key",
          tune: "Change tuning",
          gain: "Change sample gain",
          start: "Change sample start",
          end: "Change sample end",
          reverse: (reverse) =>
            reverse ? "Reverse sample" : "Unreverse sample",
          cutSelf: "Change cut mode",
          cutGroup: "Change cut group",
        },
        "Change sampler"
      )
    }

    case "setSamplerEnvelope": {
      const index = channelIndex(project, command.id)
      const { envelope } = command
      if (envelope !== undefined) {
        requireRange(envelope.attackMs, 0, 60_000, "Attack")
        requireRange(envelope.decayMs, 0, 60_000, "Decay")
        requireRange(envelope.sustain, 0, 1, "Sustain")
        requireRange(envelope.releaseMs, 0, 60_000, "Release")
      }
      const channel = project.channels[index]
      const had = channel.source.envelope !== null
      setChannel(draft, index, {
        ...channel,
        source: { ...channel.source, envelope: envelope ?? null },
      })
      if (envelope === undefined) return "Turn envelope off"
      return had ? "Change envelope" : "Turn envelope on"
    }

    case "addPattern": {
      const id = alloc(draft)
      draft.created.push(id)
      const name =
        command.name ??
        numberedName(
          "Pattern",
          project.patterns.length + 1,
          project.patterns.map((pattern) => pattern.name)
        )
      draft.project = {
        ...draft.project,
        patterns: [
          ...project.patterns,
          {
            id,
            name,
            color: colorFor(project.patterns.length),
            lengthSteps: DEFAULT_PATTERN_STEPS,
            lanes: [],
          },
        ],
      }
      draft.touched.patternList = true
      touchPattern(draft.touched, id)
      return "Add pattern"
    }

    case "removePattern": {
      patternIndex(project, command.id)
      if (project.patterns.length === 1) {
        invalid("A project needs at least one pattern")
      }
      const clips = project.playlist.clips.filter(
        (clip) => clip.content.pattern !== command.id
      )
      if (clips.length !== project.playlist.clips.length) {
        draft.touched.playlist = true
      }
      draft.project = {
        ...project,
        patterns: project.patterns.filter(
          (pattern) => pattern.id !== command.id
        ),
        playlist: draft.touched.playlist
          ? { ...project.playlist, clips }
          : project.playlist,
      }
      draft.touched.patternList = true
      touchPattern(draft.touched, command.id)
      return "Remove pattern"
    }

    case "duplicatePattern": {
      const index = patternIndex(project, command.id)
      const original = project.patterns[index]
      const id = alloc(draft)
      draft.created.push(id)
      const copy: Pattern = {
        ...original,
        id,
        name: copyName(
          original.name,
          project.patterns.map((pattern) => pattern.name)
        ),
        lanes: original.lanes.map((lane) => ({
          channel: lane.channel,
          notes: lane.notes.map((note) => ({ ...note, id: alloc(draft) })),
        })),
      }
      const patterns = [...project.patterns]
      patterns.splice(index + 1, 0, copy)
      draft.project = { ...draft.project, patterns }
      draft.touched.patternList = true
      touchPattern(draft.touched, id)
      return "Duplicate pattern"
    }

    case "movePattern": {
      const index = patternIndex(project, command.id)
      requireInteger(
        command.index,
        0,
        project.patterns.length - 1,
        "Pattern position"
      )
      draft.project = {
        ...project,
        patterns: moved(project.patterns, index, command.index),
      }
      draft.touched.patternList = true
      return "Move pattern"
    }

    case "updatePattern": {
      const index = patternIndex(project, command.id)
      const { patch } = command
      if (patch.lengthSteps !== undefined) {
        requireInteger(
          patch.lengthSteps,
          1,
          MAX_PATTERN_STEPS,
          "Pattern length"
        )
      }
      if (patch.color !== undefined) {
        requireInteger(patch.color, 0, 0xffffff, "Color")
      }
      const pattern = project.patterns[index]
      setPattern(draft, index, {
        ...pattern,
        name: patch.name ?? pattern.name,
        color: patch.color ?? pattern.color,
        lengthSteps: patch.lengthSteps ?? pattern.lengthSteps,
      })
      return patchLabel(
        patch,
        {
          name: "Rename pattern",
          color: "Change pattern color",
          lengthSteps: "Change pattern length",
        },
        "Change pattern"
      )
    }

    case "toggleStep": {
      const index = patternIndex(project, command.pattern)
      channelIndex(project, command.channel)
      requireInteger(command.step, 0, MAX_PATTERN_STEPS - 1, "Step")
      const start = command.step * TICKS_PER_STEP
      const pattern = project.patterns[index]
      const lane = pattern.lanes.find(
        (item) => item.channel === command.channel
      )
      const isOn = lane?.notes.some((note) => note.start === start) ?? false
      if (isOn) {
        setPattern(
          draft,
          index,
          withLane(pattern, command.channel, (notes) =>
            notes.filter((note) => note.start !== start)
          )
        )
        return "Clear step"
      }
      const id = alloc(draft)
      draft.created.push(id)
      setPattern(
        draft,
        index,
        withLane(pattern, command.channel, (notes) => [
          ...notes,
          {
            id,
            start,
            length: TICKS_PER_STEP,
            key: DEFAULT_KEY,
            velocity: DEFAULT_VELOCITY,
            pan: 0,
          },
        ])
      )
      return "Set step"
    }

    case "addNotes": {
      const index = patternIndex(project, command.pattern)
      channelIndex(project, command.channel)
      const added = command.notes.map((init) => {
        const note = {
          start: init.start,
          length: init.length,
          key: init.key,
          velocity: init.velocity ?? DEFAULT_VELOCITY,
          pan: init.pan ?? 0,
        }
        validateNote(note)
        return note
      })
      const notes: Note[] = added.map((note) => {
        const id = alloc(draft)
        draft.created.push(id)
        return { id, ...note }
      })
      setPattern(
        draft,
        index,
        withLane(project.patterns[index], command.channel, (existing) => [
          ...existing,
          ...notes,
        ])
      )
      return notes.length === 1 ? "Add note" : "Add notes"
    }

    case "removeNotes": {
      const index = patternIndex(project, command.pattern)
      channelIndex(project, command.channel)
      const pattern = project.patterns[index]
      const lane = pattern.lanes.find(
        (item) => item.channel === command.channel
      )
      for (const id of command.notes) {
        if (!lane?.notes.some((note) => note.id === id)) notFound("note", id)
      }
      setPattern(
        draft,
        index,
        withLane(pattern, command.channel, (notes) =>
          notes.filter((note) => !command.notes.includes(note.id))
        )
      )
      return command.notes.length === 1 ? "Remove note" : "Remove notes"
    }

    case "updateNotes": {
      const index = patternIndex(project, command.pattern)
      channelIndex(project, command.channel)
      const pattern = project.patterns[index]
      const lane = pattern.lanes.find(
        (item) => item.channel === command.channel
      )
      for (const update of command.updates) {
        if (!lane?.notes.some((note) => note.id === update.id)) {
          notFound("note", update.id)
        }
      }
      setPattern(
        draft,
        index,
        withLane(pattern, command.channel, (notes) =>
          notes.map((note) =>
            command.updates
              .filter((update) => update.id === note.id)
              .reduce(
                (current, update) => patchNote(current, update.patch),
                note
              )
          )
        )
      )
      return command.updates.length === 1 ? "Change note" : "Change notes"
    }

    case "clearLane": {
      const index = patternIndex(project, command.pattern)
      channelIndex(project, command.channel)
      setPattern(
        draft,
        index,
        withLane(project.patterns[index], command.channel, () => [])
      )
      return "Clear notes"
    }

    case "addMixerTrack": {
      const tracks = project.mixer.tracks
      const name =
        command.name ??
        numberedName(
          "Insert",
          tracks.length,
          tracks.map((track) => track.name)
        )
      draft.created.push(addMixerTrack(draft, name))
      return "Add mixer track"
    }

    case "removeMixerTrack": {
      trackIndex(project, command.id)
      if (command.id === MASTER_TRACK) {
        invalid("The master track cannot be removed")
      }
      const tracks = project.mixer.tracks
        .filter((track) => track.id !== command.id)
        .map((track) => {
          const reroute = track.output === command.id
          const dropSend = track.sends.some(
            (send) => send.target === command.id
          )
          if (!reroute && !dropSend) return track
          return {
            ...track,
            output: reroute ? MASTER_TRACK : track.output,
            sends: track.sends.filter((send) => send.target !== command.id),
          }
        })
      setTracks(draft, tracks)
      if (
        project.channels.some((channel) => channel.mixerTrack === command.id)
      ) {
        draft.project = {
          ...draft.project,
          channels: project.channels.map((channel) =>
            channel.mixerTrack === command.id
              ? { ...channel, mixerTrack: MASTER_TRACK }
              : channel
          ),
        }
        draft.touched.channels = true
      }
      return "Remove mixer track"
    }

    case "updateMixerTrack": {
      const index = trackIndex(project, command.id)
      const { patch } = command
      if (patch.volume !== undefined) {
        requireRange(patch.volume, 0, MAX_GAIN, "Track volume")
      }
      if (patch.pan !== undefined) requireRange(patch.pan, -1, 1, "Pan")
      if (patch.color !== undefined) {
        requireInteger(patch.color, 0, 0xffffff, "Color")
      }
      const track = project.mixer.tracks[index]
      setTracks(
        draft,
        replaceAt(project.mixer.tracks, index, {
          ...track,
          name: patch.name ?? track.name,
          color: patch.color ?? track.color,
          volume: patch.volume ?? track.volume,
          pan: patch.pan ?? track.pan,
          muted: patch.muted ?? track.muted,
          solo: patch.solo ?? track.solo,
        })
      )
      return patchLabel(
        patch,
        {
          name: "Rename mixer track",
          color: "Change track color",
          volume: "Change track volume",
          pan: "Change track pan",
          muted: (muted) => (muted ? "Mute track" : "Unmute track"),
          solo: (solo) => (solo ? "Solo track" : "Unsolo track"),
        },
        "Change mixer track"
      )
    }

    case "setTrackOutput": {
      const index = trackIndex(project, command.id)
      if (command.id === MASTER_TRACK) {
        invalid("The master track has no output to change")
      }
      const output = command.output ?? null
      const track = project.mixer.tracks[index]
      const tracks = replaceAt(project.mixer.tracks, index, {
        ...track,
        output,
      })
      if (output !== null) {
        trackIndex(project, output)
        if (reaches(tracks, output, command.id)) {
          invalid("That routing would feed the track back into itself")
        }
      }
      setTracks(draft, tracks)
      return "Route mixer track"
    }

    case "setSend": {
      const index = trackIndex(project, command.from)
      trackIndex(project, command.to)
      const track = project.mixer.tracks[index]
      const others = track.sends.filter((send) => send.target !== command.to)
      if (command.gain === undefined) {
        setTracks(
          draft,
          replaceAt(project.mixer.tracks, index, { ...track, sends: others })
        )
        return "Remove send"
      }
      requireRange(command.gain, 0, MAX_GAIN, "Send level")
      const existed = others.length !== track.sends.length
      const send = { target: command.to, gain: command.gain }
      const sends = existed
        ? track.sends.map((item) => (item.target === command.to ? send : item))
        : [...track.sends, send]
      if (reaches(project.mixer.tracks, command.to, command.from)) {
        invalid("That send would feed the track back into itself")
      }
      setTracks(
        draft,
        replaceAt(project.mixer.tracks, index, { ...track, sends })
      )
      return existed ? "Change send level" : "Add send"
    }

    case "addPlaylistTrack": {
      const tracks = project.playlist.tracks
      const id = alloc(draft)
      draft.created.push(id)
      const name =
        command.name ??
        numberedName(
          "Track",
          tracks.length + 1,
          tracks.map((track) => track.name)
        )
      draft.project = {
        ...draft.project,
        playlist: {
          ...project.playlist,
          tracks: [...tracks, { id, name, muted: false }],
        },
      }
      draft.touched.playlist = true
      return "Add playlist track"
    }

    case "removePlaylistTrack": {
      if (!project.playlist.tracks.some((track) => track.id === command.id)) {
        notFound("playlist track", command.id)
      }
      draft.project = {
        ...project,
        playlist: {
          tracks: project.playlist.tracks.filter(
            (track) => track.id !== command.id
          ),
          clips: project.playlist.clips.filter(
            (clip) => clip.track !== command.id
          ),
        },
      }
      draft.touched.playlist = true
      return "Remove playlist track"
    }

    case "updatePlaylistTrack": {
      const { patch } = command
      if (!project.playlist.tracks.some((track) => track.id === command.id)) {
        notFound("playlist track", command.id)
      }
      draft.project = {
        ...project,
        playlist: {
          ...project.playlist,
          tracks: project.playlist.tracks.map((track) =>
            track.id === command.id
              ? {
                  ...track,
                  name: patch.name ?? track.name,
                  muted: patch.muted ?? track.muted,
                }
              : track
          ),
        },
      }
      draft.touched.playlist = true
      return patchLabel(
        patch,
        {
          name: "Rename playlist track",
          muted: (muted) =>
            muted ? "Mute playlist track" : "Unmute playlist track",
        },
        "Change playlist track"
      )
    }

    case "addClips": {
      const prepared = command.clips.map((init) => {
        if (!project.playlist.tracks.some((track) => track.id === init.track)) {
          notFound("playlist track", init.track)
        }
        const pattern =
          project.patterns[patternIndex(project, init.content.pattern)]
        const length = init.length ?? pattern.lengthSteps * TICKS_PER_STEP
        requireInteger(init.start, 0, 0xffffffff, "Clip start")
        requireInteger(length, 1, 0xffffffff, "Clip length")
        return { ...init, length }
      })
      const clips: Clip[] = prepared.map((init) => {
        const id = alloc(draft)
        draft.created.push(id)
        return {
          id,
          track: init.track,
          start: init.start,
          length: init.length,
          offset: 0,
          muted: false,
          content: init.content,
        }
      })
      draft.project = {
        ...draft.project,
        playlist: {
          ...project.playlist,
          clips: sortClips([...project.playlist.clips, ...clips]),
        },
      }
      draft.touched.playlist = true
      return clips.length === 1 ? "Add clip" : "Add clips"
    }

    case "removeClips": {
      for (const id of command.clips) {
        if (!project.playlist.clips.some((clip) => clip.id === id)) {
          notFound("clip", id)
        }
      }
      draft.project = {
        ...project,
        playlist: {
          ...project.playlist,
          clips: project.playlist.clips.filter(
            (clip) => !command.clips.includes(clip.id)
          ),
        },
      }
      draft.touched.playlist = true
      return command.clips.length === 1 ? "Remove clip" : "Remove clips"
    }

    case "updateClips": {
      let clips = project.playlist.clips
      for (const update of command.updates) {
        const index = clips.findIndex((clip) => clip.id === update.id)
        if (index < 0) notFound("clip", update.id)
        const clip = clips[index]
        const next: Clip = {
          ...clip,
          track: update.patch.track ?? clip.track,
          start: update.patch.start ?? clip.start,
          length: update.patch.length ?? clip.length,
          offset: update.patch.offset ?? clip.offset,
          muted: update.patch.muted ?? clip.muted,
        }
        if (!project.playlist.tracks.some((track) => track.id === next.track)) {
          notFound("playlist track", next.track)
        }
        requireInteger(next.start, 0, 0xffffffff, "Clip start")
        requireInteger(next.length, 1, 0xffffffff, "Clip length")
        requireInteger(next.offset, 0, 0xffffffff, "Clip offset")
        clips = replaceAt(clips, index, next)
      }
      draft.project = {
        ...project,
        playlist: { ...project.playlist, clips: sortClips(clips) },
      }
      draft.touched.playlist = true
      return command.updates.length === 1 ? "Change clip" : "Change clips"
    }

    case "batch": {
      if (command.commands.length === 0) {
        invalid("A batch needs at least one command")
      }
      const labels = command.commands.map((inner) => run(draft, inner))
      return command.label ?? labels[0]
    }
  }
}
