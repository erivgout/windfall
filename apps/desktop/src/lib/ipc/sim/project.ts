import type { Command, Project } from "@/bindings"
import { FORMAT_VERSION, MASTER_TRACK } from "@/lib/units"

import { applyCommand } from "./commands"

/** What "New project" gives: a master track, one empty pattern, no channels. */
export function newProject(name = "Untitled"): Project {
  return {
    formatVersion: FORMAT_VERSION,
    nextId: 2,
    settings: {
      name,
      tempoBpm: 140,
      timeSignature: { numerator: 4, denominator: 4 },
      swing: 0,
    },
    samples: [],
    channels: [],
    patterns: [
      { id: 1, name: "Pattern 1", color: 0xe5488f, lengthSteps: 16, lanes: [] },
    ],
    mixer: {
      tracks: [
        {
          id: MASTER_TRACK,
          name: "Master",
          color: 0x9ca3af,
          volume: 1,
          pan: 0,
          muted: false,
          solo: false,
          output: null,
          sends: [],
        },
      ],
    },
    playlist: { tracks: [], clips: [] },
  }
}

const DEMO_KIT = [
  { name: "Kick", file: "Drums/Kicks/Kick 01.wav", steps: [0, 4, 8, 12] },
  { name: "Clap", file: "Drums/Claps/Clap 01.wav", steps: [4, 12] },
  {
    name: "Hat",
    file: "Drums/Hats/Closed Hat 01.wav",
    steps: [0, 2, 4, 6, 8, 10, 12, 14],
  },
  { name: "Snare", file: "Drums/Snares/Snare 01.wav", steps: [7, 15] },
]

/** The project the mock starts with: four sampler channels and a simple beat. */
export function demoProject(): Project {
  let project = newProject("Demo beat")
  const run = (command: Command) => {
    const applied = applyCommand(project, command)
    project = applied.project
    return applied.created
  }

  run({ type: "updateSettings", patch: { tempoBpm: 128 } })
  const pattern = project.patterns[0].id
  for (const part of DEMO_KIT) {
    const [sample] = run({
      type: "addSample",
      name: part.name,
      path: { kind: "factory", path: part.file },
    })
    const [channel] = run({ type: "addChannel", name: part.name, sample })
    for (const step of part.steps) {
      run({ type: "toggleStep", pattern, channel, step })
    }
  }
  return project
}
