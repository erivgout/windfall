import type { Command, Project } from "@/bindings"

import { SimDocument } from "./document"
import { sim } from "./wasm"

/** The model's empty project: one empty pattern and only the master track. */
export function emptyProject(name = "Untitled"): Project {
  return sim.call<Project>("project_new", 0, name)
}

/**
 * What `base` becomes once `build` has run its commands on it. `run`
 * resolves to the ids a command created. Only the project comes back, so a
 * document made from it starts clean, with the commands outside its undo
 * history.
 */
export function buildProject(
  base: Project,
  build: (run: (command: Command) => number[]) => void
): Project {
  const scratch = SimDocument.create(base)
  try {
    build((command) => scratch.dispatch(command).created)
    return scratch.project()
  } finally {
    scratch.dispose()
  }
}

/**
 * The sounds a new project starts with and their channel volumes: the
 * factory's default kit and the shell's `KIT_VOLUMES`, which leave about
 * 6 dB of headroom when all four land on one step.
 */
const STARTER_KIT = [
  { name: "Kick Punch", file: "Drums/Kicks/Kick Punch.wav", volume: 0.36 },
  { name: "Clap Wide", file: "Drums/Claps/Clap Wide.wav", volume: 0.25 },
  { name: "Hat Closed 1", file: "Drums/Hats/Hat Closed 1.wav", volume: 0.18 },
  { name: "Snare Tight", file: "Drums/Snares/Snare Tight.wav", volume: 0.29 },
]

type KitSound = { name: string; file: string; volume: number }

/** Adds a sampler channel that plays a factory sound. Resolves to its id. */
function addKitChannel(
  run: (command: Command) => number[],
  sound: KitSound
): number {
  const [sample] = run({
    type: "addSample",
    name: sound.name,
    path: { kind: "factory", path: sound.file },
  })
  const [channel] = run({ type: "addChannel", name: sound.name, sample })
  run({
    type: "updateChannel",
    id: channel,
    patch: { volume: sound.volume },
  })
  return channel
}

/** What "New project" gives, as in the app: empty, plus the starter kit. */
export function starterProject(): Project {
  return buildProject(emptyProject(), (run) => {
    for (const sound of STARTER_KIT) addKitChannel(run, sound)
  })
}

const DEMO_KIT = [
  {
    name: "Kick",
    file: "Drums/Kicks/Kick 01.wav",
    volume: 0.36,
    steps: [0, 4, 8, 12],
  },
  {
    name: "Clap",
    file: "Drums/Claps/Clap 01.wav",
    volume: 0.25,
    steps: [4, 12],
  },
  {
    name: "Hat",
    file: "Drums/Hats/Closed Hat 01.wav",
    volume: 0.18,
    steps: [0, 2, 4, 6, 8, 10, 12, 14],
  },
  {
    name: "Snare",
    file: "Drums/Snares/Snare 01.wav",
    volume: 0.29,
    steps: [7, 15],
  },
]

/**
 * The project the mock starts with, so a browser opens on something to
 * hear and see: four sampler channels and a simple beat.
 */
export function demoProject(): Project {
  const base = emptyProject("Demo beat")
  const pattern = base.patterns[0].id
  return buildProject(base, (run) => {
    run({ type: "updateSettings", patch: { tempoBpm: 128 } })
    for (const part of DEMO_KIT) {
      const channel = addKitChannel(run, part)
      for (const step of part.steps) {
        run({ type: "toggleStep", pattern, channel, step })
      }
    }
  })
}
