import { act, fireEvent, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import type { ReactNode } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { EffectKind } from "@/bindings"
import { ValueContextMenus } from "@/components/value-context-menu"
import MixerPanel from "@/features/mixer"
import { chain, kinds } from "@/features/mixer/effect-test-utils"
import { useEffectsUi } from "@/features/mixer/effects-ui"
import {
  flush,
  history,
  project,
  strip,
  stubCanvas,
  trackNamed,
} from "@/features/mixer/test-utils"
import { effectDescriptor, readParam } from "@/features/params"
import { registry, runAction } from "@/lib/actions"
import { SimDocument } from "@/lib/ipc/sim/document"
import { emptyProject } from "@/lib/ipc/sim/project"
import { dispatch, redo, undo, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { startTestApp } from "@/test/harness"

// jsdom has no layout. Only the resize surface is substituted: descriptors,
// controls, actions, stores and the Rust WASM document are the real code.
vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizablePanel: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => <div role="separator" />,
}))
vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const KINDS = [
  "fastLowpass",
  "selectableFilter",
  "bassShelf",
] as const satisfies readonly EffectKind[]
let app: Awaited<ReturnType<typeof startTestApp>>

beforeEach(async () => {
  stubCanvas()
  useEffectsUi.setState(useEffectsUi.getInitialState(), true)
  app = await startTestApp()
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})

async function open(kind: (typeof KINDS)[number]) {
  const user = userEvent.setup()
  render(
    <ValueContextMenus>
      <MixerPanel />
    </ValueContextMenus>
  )
  const descriptor = effectDescriptor(kind)
  expect(registry.get(`mixer.addEffect.${kind}`)).toBeDefined()
  expect(registry.get(`mixer.replaceEffect.${kind}`)).toBeDefined()
  await user.click(
    within(strip("Clap")).getByRole("button", { name: "Add effect" })
  )
  await user.click(
    await screen.findByRole("menuitem", { name: descriptor.name })
  )
  await flush()
  expect(kinds("Clap")).toEqual([kind])
  expect(chain("Clap")[0]).toMatchObject({
    enabled: true,
    mix: 1,
    params: descriptor.defaults,
  })
  expect(history().cursor).toBe(1)
  return {
    user,
    descriptor,
    editor: within(
      screen.getByRole("group", { name: `${descriptor.name} settings` })
    ),
  }
}

describe("filter family with the shared Rust WASM document", () => {
  it.each(KINDS)(
    "%s retains actual mixer mix/bypass/copy settings and replaces with a fresh processor",
    async (kind) => {
      const { descriptor, editor } = await open(kind)
      const original = chain("Clap")[0].id
      const info = descriptor.params.find((info) => info.kind === "float")!
      const control = editor.getByRole("slider", { name: info.name })
      const key = info.default === info.max ? "Home" : "End"
      fireEvent.keyDown(control, { key })
      fireEvent.keyUp(control, { key })
      const mix = screen.getByRole("slider", {
        name: `${descriptor.name} dry/wet mix`,
      })
      fireEvent.keyDown(mix, { key: "Home" })
      fireEvent.keyUp(mix, { key: "Home" })
      await flush()
      expect(chain("Clap")[0].mix).toBe(0)
      const before = structuredClone(chain("Clap")[0])
      await runAction("mixer.bypassEffect")
      await flush()
      expect(chain("Clap")[0].enabled).toBe(false)
      await undo()
      await flush()
      expect(chain("Clap")[0]).toEqual(before)
      await runAction("mixer.duplicateEffect")
      await flush()
      expect(chain("Clap")).toHaveLength(2)
      expect(chain("Clap")[1].id).not.toBe(original)
      expect(chain("Clap")[1]).toMatchObject({
        params: before.params,
        mix: 0,
        enabled: true,
      })
      await undo()
      await flush()
      expect(chain("Clap")).toEqual([before])
      act(() => useEffectsUi.getState().selectEffect(original))
      const replacement = KINDS[(KINDS.indexOf(kind) + 1) % KINDS.length]
      await runAction(`mixer.replaceEffect.${replacement}`)
      await flush()
      expect(kinds("Clap")).toEqual([replacement])
      expect(chain("Clap")[0].id).not.toBe(original)
      expect(chain("Clap")[0].params).toEqual(
        effectDescriptor(replacement).defaults
      )
      expect(
        screen.getByRole("group", {
          name: `${effectDescriptor(replacement).name} settings`,
        })
      ).toBeVisible()
      await undo()
      await flush()
      expect(chain("Clap")).toEqual([before])
    }
  )

  it.each(KINDS)(
    "%s exposes every generic control, history, reset, noop and save/reopen",
    async (kind) => {
      const { descriptor, editor } = await open(kind)
      const track = trackNamed("Clap").id
      const effect = chain("Clap")[0].id
      for (const info of descriptor.params.filter(
        (info) => info.kind === "float"
      )) {
        const control = editor.getByRole("slider", { name: info.name })
        expect(control).toHaveAttribute("aria-valuemin", String(info.min))
        expect(control).toHaveAttribute("aria-valuemax", String(info.max))
        expect(Number(control.getAttribute("aria-valuenow"))).toBeCloseTo(
          info.default,
          4
        )
        const before = chain("Clap")[0].params
        const cursor = history().cursor
        const key = info.default === info.max ? "Home" : "End"
        const value = info.default === info.max ? info.min : info.max
        fireEvent.keyDown(control, { key })
        fireEvent.keyDown(control, { key })
        fireEvent.keyUp(control, { key })
        await flush()
        expect(history().cursor).toBe(cursor + 1)
        expect(readParam(chain("Clap")[0].params, info)).toBe(value)
        await undo()
        await flush()
        expect(chain("Clap")[0].params).toEqual(before)
        await redo()
        await flush()
        expect(readParam(chain("Clap")[0].params, info)).toBe(value)
        fireEvent.doubleClick(control)
        await flush()
        expect(readParam(chain("Clap")[0].params, info)).toBeCloseTo(
          info.default,
          4
        )
        const resetCursor = history().cursor
        fireEvent.doubleClick(control)
        await flush()
        expect(history().cursor).toBe(resetCursor)
        // Leave a nondefault setting in the actual file, rather than merely
        // reopening defaults. This also exercises the generic index command.
        await dispatch({
          type: "setEffectParam",
          track,
          effect,
          param: descriptor.params.indexOf(info),
          value,
        })
        await flush()
      }
      // Patches intentionally omit the id allocator. The document snapshot
      // is the complete project that is serialized, including nextId.
      const saved = structuredClone(
        (await app.backend.documentSnapshot()).project
      )
      const path = await app.backend.projectSave(`/projects/${kind}.windfall`)
      await flush()
      expect(useProjectStore.getState().dirty).toBe(false)
      const cursor = history().cursor
      await dispatch({
        type: "setEffectParams",
        track,
        effect,
        params: chain("Clap")[0].params,
      })
      await flush()
      expect(history().cursor).toBe(cursor)
      expect(useProjectStore.getState().dirty).toBe(false)
      await dispatch({ type: "removeEffect", track, effect })
      await flush()
      expect(kinds("Clap")).toEqual([])
      await act(async () => {
        await app.backend.projectOpen(path)
      })
      await flush()
      expect(project()).toEqual(saved)
      expect(kinds("Clap")).toEqual([kind])
      expect(useProjectStore.getState().dirty).toBe(false)
    }
  )

  it("selectable filter exposes all seven real choice tags, with undo and file persistence", async () => {
    const { user, descriptor } = await open("selectableFilter")
    const info = descriptor.params[0]
    expect(info.id).toBe("mode")
    expect(info.choices.map((choice) => choice.value)).toEqual([
      "lowpass",
      "highpass",
      "bandpass",
      "notch",
      "lowShelf",
      "peak",
      "highShelf",
    ])
    for (const [index, choice] of info.choices.entries()) {
      const control = screen.getByRole("combobox", { name: info.name })
      const previous = readParam(chain("Clap")[0].params, info)
      const cursor = history().cursor
      await user.click(control)
      await user.click(
        await screen.findByRole("option", { name: choice.label })
      )
      await flush()
      expect(readParam(chain("Clap")[0].params, info)).toBe(index)
      expect(control).toHaveTextContent(choice.label)
      if (index === previous) {
        expect(history().cursor).toBe(cursor)
      } else {
        expect(history().cursor).toBe(cursor + 1)
        await undo()
        await flush()
        expect(readParam(chain("Clap")[0].params, info)).toBe(previous)
        await redo()
        await flush()
        expect(readParam(chain("Clap")[0].params, info)).toBe(index)
      }
      const saved = structuredClone(
        (await app.backend.documentSnapshot()).project
      )
      const path = await app.backend.projectSave("/projects/mode.windfall")
      await act(async () => {
        await app.backend.projectOpen(path)
      })
      await flush()
      expect(project()).toEqual(saved)
      // Opening a file clears the inspector's selection; selecting the
      // persisted slot restores the same generic editor, with the same tag.
      act(() => {
        useUiStore.getState().selectTrack(trackNamed("Clap").id)
        useEffectsUi.getState().selectEffect(chain("Clap")[0].id)
      })
      await flush()
      expect(
        screen.getByRole("combobox", { name: info.name })
      ).toHaveTextContent(choice.label)
    }
  })

  it.each(KINDS)(
    "%s makes automation from its actual control and restores it with removal undo",
    async (kind) => {
      const { user, descriptor, editor } = await open(kind)
      const info = descriptor.params.find((info) => info.kind === "float")!
      const param = descriptor.params.indexOf(info)
      const track = trackNamed("Clap").id
      const effect = chain("Clap")[0].id
      fireEvent.contextMenu(editor.getByRole("slider", { name: info.name }))
      await user.click(
        await screen.findByRole("menuitem", { name: "Create automation clip" })
      )
      await flush()
      expect(project().automations).toHaveLength(1)
      const automation = project().automations[0]
      expect(automation.target).toEqual({
        type: "effectParam",
        track,
        effect,
        param,
      })
      expect(
        project().playlist.clips.some(
          (clip) =>
            clip.content.type === "automation" &&
            clip.content.automation === automation.id
        )
      ).toBe(true)
      const saved = structuredClone(
        (await app.backend.documentSnapshot()).project
      )
      const path = await app.backend.projectSave(
        `/projects/${kind}-automation.windfall`
      )
      await dispatch({ type: "removeEffect", track, effect })
      await flush()
      expect(project().automations).toEqual([])
      expect(project().playlist.clips).toEqual([])
      await undo()
      await flush()
      expect(project().automations).toEqual(saved.automations)
      expect(project().playlist).toEqual(saved.playlist)
      await act(async () => {
        await app.backend.projectOpen(path)
      })
      await flush()
      expect(project()).toEqual(saved)
    }
  )

  it.each(KINDS)(
    "%s resolves every automation index and defaults through actual WASM",
    (kind) => {
      const doc = SimDocument.create(emptyProject())
      let reopened: SimDocument | undefined
      try {
        const descriptor = effectDescriptor(kind)
        const effect = doc.dispatch({ type: "addEffect", track: 0, kind })
          .created[0]
        const settings = () => doc.project().mixer.tracks[0].effects[0].params
        expect(settings()).toEqual(descriptor.defaults)
        for (const [param, info] of descriptor.params.entries()) {
          const value = info.default === info.max ? info.min : info.max
          const before = structuredClone(settings())
          doc.dispatch({
            type: "setEffectParam",
            track: 0,
            effect,
            param,
            value,
          })
          expect(readParam(settings(), info)).toBe(value)
          doc.undo()
          expect(settings()).toEqual(before)
          doc.redo()
          expect(readParam(settings(), info)).toBe(value)
          doc.dispatch({
            type: "addAutomation",
            target: { type: "effectParam", track: 0, effect, param },
          })
          expect(doc.project().automations.at(-1)?.target).toEqual({
            type: "effectParam",
            track: 0,
            effect,
            param,
          })
        }
        const before = structuredClone(doc.project())
        const cursor = doc.snapshot(null).history.cursor
        expect(() =>
          doc.dispatch({
            type: "setEffectParam",
            track: 0,
            effect,
            param: descriptor.params.length,
            value: 0,
          })
        ).toThrow()
        expect(doc.project()).toEqual(before)
        expect(doc.snapshot(null).history.cursor).toBe(cursor)
        reopened = SimDocument.open(doc.fileText())
        expect(reopened.project()).toEqual(before)
      } finally {
        reopened?.dispose()
        doc.dispose()
      }
    }
  )
})
