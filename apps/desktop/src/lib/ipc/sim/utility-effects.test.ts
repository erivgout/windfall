import { describe, expect, it } from "vitest"
import { effectDescriptor, readParam } from "@/features/params"
import { SimDocument } from "./document"
import { emptyProject } from "./project"
import { simulatedLatencyFrames } from "./effects"

const utilities = [
  "balance",
  "dcBlock",
  "channelMute",
  "polarity",
  "stereoMatrix",
  "softClipper",
  "distortion",
] as const

describe("utility effects in the real WASM document", () => {
  it.each(utilities)(
    "adds, edits, undoes and reopens %s in v1 files",
    (kind) => {
      const doc = SimDocument.create(emptyProject())
      try {
        const effect = doc.dispatch({ type: "addEffect", track: 0, kind })
          .created[0]
        const descriptor = effectDescriptor(kind)
        const slot = () => doc.project().mixer.tracks[0].effects[0]
        expect(slot().params).toEqual(descriptor.defaults)
        for (const [param, info] of descriptor.params.entries()) {
          const before = slot().params
          const original = readParam(before, info)
          const value = original === info.max ? info.min : info.max
          const result = doc.dispatch({
            type: "setEffectParam",
            track: 0,
            effect,
            param,
            value,
          })
          expect(result.patch.mixer).not.toBeNull()
          expect(readParam(slot().params, info)).toBe(value)
          const changed = slot().params
          doc.undo()
          expect(slot().params).toEqual(before)
          doc.redo()
          expect(slot().params).toEqual(changed)
        }
        const target = {
          type: "effectParam",
          track: 0,
          effect,
          param: 0,
        } as const
        doc.dispatch({ type: "addAutomation", target })
        expect(doc.project().automations[0].target).toEqual(target)
        const reopened = SimDocument.open(doc.fileText())
        try {
          expect(reopened.project()).toEqual(doc.project())
        } finally {
          reopened.dispose()
        }
        expect(doc.project().formatVersion).toBe(1)
        doc.dispatch({ type: "removeEffect", track: 0, effect })
        expect(doc.project().mixer.tracks[0].effects).toEqual([])
        expect(doc.project().automations).toEqual([])
        doc.undo()
        expect(slot().params.type).toBe(kind)
      } finally {
        doc.dispose()
      }
    }
  )

  it("reports the matrix common delay and FIR latency for browser UI simulation", () => {
    const doc = SimDocument.create(emptyProject())
    try {
      const effect = doc.dispatch({
        type: "addEffect",
        track: 0,
        kind: "stereoMatrix",
      }).created[0]
      doc.dispatch({
        type: "setEffectParams",
        track: 0,
        effect,
        params: {
          ...effectDescriptor("stereoMatrix").defaults,
          leftDelayMs: 2,
          rightDelayMs: 5,
        },
      })
      doc.dispatch({ type: "addEffect", track: 0, kind: "distortion" })
      expect(simulatedLatencyFrames(doc.project(), 48_000)).toBe(96 + 32)
      doc.dispatch({
        type: "updateEffect",
        track: 0,
        effect,
        patch: { enabled: false },
      })
      expect(simulatedLatencyFrames(doc.project(), 48_000)).toBe(96 + 32)
    } finally {
      doc.dispose()
    }
  })
})
