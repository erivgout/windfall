import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import type { PluginBinding, PluginManagerState } from "@/bindings"
import { targetState } from "@/lib/automation/targets"
import { useProjectStore, useUiStore } from "@/lib/store"
import { dispatch, undo } from "@/lib/store/project"
import { startTestApp } from "@/test/harness"
import { useRackStore } from "@/features/channel-rack/rack-store"
import { useEffectsUi } from "@/features/mixer/effects-ui"
import { PluginControls } from "./controls"
import { PluginManager } from "./manager"
import { openPluginManager, usePluginUi } from "./store"

const binding = (): PluginBinding => ({
  target: { type: "instrument", channel: 0 },
  format: "clap",
  path: "/plugins/native.clap",
  id: "native.synth",
  name: "Native Synth",
  state: [1, 2],
  parameters: [
    {
      id: 43,
      name: "Native cutoff",
      min: 100,
      max: 8000,
      value: 400,
      stepped: false,
      readOnly: false,
      automatable: true,
    },
  ],
})
const catalog = (): PluginManagerState => ({
  folders: ["/plugins"],
  entries: [
    {
      path: "/plugins/native.clap",
      format: "clap",
      id: "native.synth",
      name: "Native Synth",
      vendor: "Fixture",
      instrument: true,
      usable: true,
      error: null,
    },
    {
      path: "/plugins/gain.clap",
      format: "clap",
      id: "native.gain",
      name: "Native Gain",
      vendor: "Fixture",
      instrument: false,
      usable: true,
      error: null,
    },
  ],
  blocked: [
    {
      path: "/plugins/broken.clap",
      format: "clap",
      id: "",
      name: "Broken",
      vendor: "",
      instrument: false,
      usable: false,
      error: "Scanner timed out",
    },
  ],
  scanning: false,
  completed: 3,
  total: 3,
  current: null,
  error: null,
  instances: [],
})
let app: Awaited<ReturnType<typeof startTestApp>>
beforeEach(async () => {
  app = await startTestApp()
  usePluginUi.setState({ open: false, track: undefined })
  Object.defineProperty(app.backend, "kind", { value: "tauri" })
  vi.spyOn(app.backend, "pluginsState").mockResolvedValue(catalog())
  vi.spyOn(app.backend, "pluginsScan").mockResolvedValue()
  vi.spyOn(app.backend, "pluginEditor").mockResolvedValue()
  vi.spyOn(app.backend, "pluginsAdd").mockImplementation((_path, id, track) =>
    app.backend.dispatch(
      track === undefined
        ? { type: "addPluginInstrument", plugin: binding() }
        : {
            type: "addPluginEffect",
            track,
            plugin: {
              ...binding(),
              id,
              name: "Native Gain",
              target: { type: "effect", effect: 0 },
            },
          }
    )
  )
})
afterEach(() => {
  app.stop()
  vi.restoreAllMocks()
})

it("searches scanned identities, retries blocked files, and selects an added instrument", async () => {
  act(() => openPluginManager())
  render(<PluginManager />)
  await screen.findByText("Native Synth · Fixture · CLAP")
  await userEvent.click(screen.getByRole("button", { name: "Retry Broken" }))
  expect(app.backend.pluginsScan).toHaveBeenCalledWith("/plugins/broken.clap")
  await userEvent.type(screen.getByLabelText("Search plugins"), "Synth")
  expect(
    screen.queryByText("Native Gain · Fixture · CLAP")
  ).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole("button", { name: "Add instrument" }))
  await waitFor(() => expect(usePluginUi.getState().open).toBe(false))
  const plugin = useProjectStore.getState().project.plugins?.[0]
  expect(plugin?.name).toBe("Native Synth")
  expect(useUiStore.getState().selectedChannel).toBe(
    plugin?.target.type === "instrument" ? plugin.target.channel : -1
  )
  expect(useRackStore.getState().inspectorOpen).toBe(true)
})

it("adds an effect to the requested track and opens its actual inspector", async () => {
  act(() => openPluginManager(0))
  render(<PluginManager />)
  await screen.findByText("Native Gain · Fixture · CLAP")
  await userEvent.click(screen.getByRole("button", { name: "Add effect" }))
  await waitFor(() => expect(usePluginUi.getState().open).toBe(false))
  const plugin = useProjectStore.getState().project.plugins?.[0]
  expect(app.backend.pluginsAdd).toHaveBeenCalledWith(
    "/plugins/gain.clap",
    "native.gain",
    0
  )
  expect(useEffectsUi.getState().selectedEffect).toBe(
    plugin?.target.type === "effect" ? plugin.target.effect : -1
  )
})

it("edits stable native IDs in their real range with undo and editor routing", async () => {
  await act(async () => {
    await dispatch({ type: "addPluginInstrument", plugin: binding() })
  })
  const plugin = useProjectStore.getState().project.plugins![0]
  render(<PluginControls binding={plugin} />)
  const cutoff = screen.getByLabelText("Native cutoff")
  expect(cutoff).toHaveAttribute("min", "100")
  expect(cutoff).toHaveAttribute("max", "8000")
  fireEvent.focus(cutoff)
  fireEvent.change(cutoff, { target: { value: "9000" } })
  fireEvent.blur(cutoff)
  await waitFor(() =>
    expect(
      useProjectStore.getState().project.plugins![0].parameters[0].value
    ).toBe(8000)
  )
  const target =
    plugin.target.type === "instrument"
      ? {
          type: "instrumentParam" as const,
          channel: plugin.target.channel,
          param: 0,
        }
      : { type: "tempo" as const }
  expect(
    targetState(useProjectStore.getState().project, target)?.range
  ).toEqual({ min: 100, max: 8000, taper: "linear" })
  await act(async () => {
    await undo()
  })
  expect(
    useProjectStore.getState().project.plugins![0].parameters[0].value
  ).toBe(400)
  await userEvent.click(
    screen.getByRole("button", { name: "Open native editor" })
  )
  expect(app.backend.pluginEditor).toHaveBeenCalledWith(plugin.target, true)
})
