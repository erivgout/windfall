import { act, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TooltipProvider } from "@/components/ui/tooltip"
import BrowserPanel from "@/features/browser"
import { registerBrowserActions } from "@/features/browser/actions"
import ChannelRackPanel from "@/features/channel-rack"
import { registerChannelRackActions } from "@/features/channel-rack/actions"
import MixerPanel from "@/features/mixer"
import { registerMixerActions } from "@/features/mixer/actions"
import { useMixerUi } from "@/features/mixer/mixer-ui"
import PianoRollPanel from "@/features/piano-roll"
import { registerPianoRollActions } from "@/features/piano-roll/actions"
import { currentSession } from "@/features/piano-roll/session"
import { usePianoRollStore } from "@/features/piano-roll/store"
import { registerPlaylistActions } from "@/features/playlist/actions"
import { TransportBar } from "@/features/transport/transport-bar"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

/*
 * The repros of the keyboard bugs QA found in the running app, each driven
 * the way a person does it: click somewhere, then press a key.
 */

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let stops: (() => void)[] = []

beforeEach(async () => {
  useMixerUi.setState(useMixerUi.getInitialState(), true)
  usePianoRollStore.setState(usePianoRollStore.getInitialState(), true)
  const app = await startTestApp()
  stops = [
    registerBrowserActions(),
    registerChannelRackActions(),
    registerMixerActions(),
    registerPianoRollActions(),
    registerPlaylistActions(),
    app.stop,
  ]
})
afterEach(() => {
  for (const stop of stops) stop()
})

const project = () => useProjectStore.getState().project
const history = () => useProjectStore.getState().history
const playing = () => useTransportStore.getState().playing
const prompts = () => usePromptStore.getState()
const flush = () => act(settle)

/** The window without the docks: transport, one editor, mixer and browser. */
function renderApp(editor: "rack" | "pianoRoll" = "rack") {
  if (editor === "pianoRoll") {
    act(() => useUiStore.getState().showCenterTab("pianoRoll"))
  }
  return render(
    <TooltipProvider>
      <TransportBar />
      {editor === "rack" ? <ChannelRackPanel /> : <PianoRollPanel />}
      <MixerPanel />
      <BrowserPanel />
    </TooltipProvider>
  )
}

const transportButton = (name: string) =>
  within(screen.getByRole("toolbar", { name: "Transport" })).getByRole(
    "button",
    { name }
  )
const strip = (name: string) =>
  screen.getByRole("group", { name: new RegExp(`^${name}, track`) })
const channelButton = (name: string) => screen.getByRole("button", { name })
/** The notes of the first channel, which the piano roll opens on. */
const kickNotes = () =>
  project().patterns[0].lanes.find(
    (lane) => lane.channel === project().channels[0].id
  )?.notes ?? []

describe("Space after a click on a button", () => {
  it("plays after Stop was clicked", async () => {
    const user = userEvent.setup()
    renderApp()
    await user.click(transportButton("Stop"))
    expect(transportButton("Stop")).toHaveFocus()

    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(true)
    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(false)
  })

  it("plays after Undo was clicked, and does not undo a second time", async () => {
    const user = userEvent.setup()
    await dispatch({ type: "addPattern" })
    await dispatch({ type: "addPattern" })
    renderApp()

    await user.click(transportButton("Undo"))
    await flush()
    expect(history().cursor).toBe(1)
    expect(transportButton("Undo")).toHaveFocus()

    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(true)
    expect(history().cursor).toBe(1)
    expect(project().patterns).toHaveLength(2)
  })

  it("plays from a mute button, a fader and a step without changing them", async () => {
    const user = userEvent.setup()
    renderApp()
    const mute = within(strip("Kick")).getByRole("button", { name: "Mute" })
    await user.click(mute)
    await flush()
    expect(project().mixer.tracks[1].muted).toBe(true)

    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(true)
    expect(project().mixer.tracks[1].muted).toBe(true)

    act(() =>
      within(strip("Hat")).getByRole("slider", { name: "Hat volume" }).focus()
    )
    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(false)
    expect(project().mixer.tracks[3].volume).toBe(1)

    const before = history().cursor
    const step = screen
      .getByRole("group", { name: "Kick steps" })
      .querySelector<HTMLElement>("[data-step='2']")
    if (!step) throw new Error("no step")
    act(() => step.focus())
    await user.keyboard(" ")
    await flush()
    expect(playing()).toBe(true)
    expect(history().cursor).toBe(before)
  })
})

describe("Delete goes to the panel that has the keyboard", () => {
  it("does not delete a channel from the mixer or the browser", async () => {
    const user = userEvent.setup()
    renderApp()
    // A channel is selected in the rack, as it is after any click there.
    await user.click(channelButton("Kick"))
    expect(useUiStore.getState().selectedChannel).not.toBeNull()

    act(() => strip("Clap").focus())
    await user.keyboard("{Delete}")
    await flush()
    // The mixer asks about its own track, which a channel plays into.
    expect(prompts().confirm?.title).toBe('Delete the track "Clap"?')
    act(() => prompts().confirm?.resolve(null))
    expect(project().channels).toHaveLength(4)
    expect(project().mixer.tracks).toHaveLength(5)

    act(() => screen.getByRole("tree").focus())
    await user.keyboard("{Delete}{Backspace}")
    await flush()
    expect(prompts().confirm).toBeNull()
    expect(project().channels).toHaveLength(4)
  })

  it("deletes the selected channel from the rack, after asking", async () => {
    const user = userEvent.setup()
    renderApp()
    await user.click(channelButton("Kick"))
    await user.keyboard("{Delete}")
    await flush()
    expect(prompts().confirm?.title).toBe("Delete Kick?")
    act(() => prompts().confirm?.resolve("delete"))
    await flush()
    expect(project().channels.map((channel) => channel.name)).toEqual([
      "Clap",
      "Hat",
      "Snare",
    ])
  })

  it("deletes the selected notes in the piano roll, and no channel", async () => {
    const user = userEvent.setup()
    renderApp("pianoRoll")
    await flush()
    const notes = kickNotes
    expect(notes().length).toBeGreaterThan(0)

    await user.click(screen.getByRole("application", { name: "Note grid" }))
    await user.keyboard("{Control>}a{/Control}")
    expect(currentSession()?.editor.selection.size).toBe(notes().length)
    await user.keyboard("{Delete}")
    await flush()

    expect(notes()).toEqual([])
    expect(prompts().confirm).toBeNull()
    expect(project().channels).toHaveLength(4)
  })

  it("keeps deleting notes after a click on a transport button", async () => {
    const user = userEvent.setup()
    renderApp("pianoRoll")
    await flush()
    await user.click(screen.getByRole("application", { name: "Note grid" }))
    act(() => currentSession()?.editor.selectAll())
    await user.click(transportButton("Stop"))
    expect(transportButton("Stop")).toHaveFocus()
    await user.keyboard("{Delete}")
    await flush()
    expect(kickNotes()).toEqual([])
  })
})

describe("F2 renames what is selected in the panel that has the keyboard", () => {
  for (const preset of ["windfall", "fl"] as const) {
    it(`renames the channel in the rack and the track in the mixer (${preset} keys)`, async () => {
      const user = userEvent.setup()
      act(() => useUiStore.getState().setKeymap(preset))
      renderApp()
      await user.click(channelButton("Hat"))
      await user.keyboard("{F2}")
      await flush()
      expect(prompts().text?.title).toBe("Rename channel")
      expect(prompts().text?.initial).toBe("Hat")
      act(() => prompts().text?.resolve(null))

      act(() => strip("Snare").focus())
      await user.keyboard("{F2}")
      await flush()
      expect(prompts().text).toBeNull()
      expect(screen.getByRole("textbox", { name: "Track name" })).toHaveValue(
        "Snare"
      )
    })
  }

  it("renames the pattern in the FL preset where the panel has nothing of its own", async () => {
    const user = userEvent.setup()
    act(() => useUiStore.getState().setKeymap("fl"))
    renderApp()
    act(() => screen.getByRole("tree").focus())
    await user.keyboard("{F2}")
    await flush()
    expect(prompts().text?.title).toBe("Rename pattern")
  })
})

describe("the same key in different panels", () => {
  it("duplicates the channel with Ctrl+D in the rack and nothing in the mixer", async () => {
    const user = userEvent.setup()
    renderApp()
    act(() => strip("Kick").focus())
    await user.keyboard("{Control>}d{/Control}")
    await flush()
    expect(project().channels).toHaveLength(4)

    await user.click(channelButton("Kick"))
    await user.keyboard("{Control>}d{/Control}")
    await flush()
    expect(project().channels).toHaveLength(5)
  })

  it("solos a track with S in the mixer and picks a tool with S in the piano roll", async () => {
    const user = userEvent.setup()
    renderApp("pianoRoll")
    await flush()
    act(() => strip("Hat").focus())
    await user.keyboard("s")
    await flush()
    expect(project().mixer.tracks[3].solo).toBe(true)
    expect(usePianoRollStore.getState().tool).toBe("draw")

    await user.click(screen.getByRole("application", { name: "Note grid" }))
    await user.keyboard("s")
    await flush()
    expect(usePianoRollStore.getState().tool).toBe("select")
    expect(project().mixer.tracks[3].solo).toBe(true)
  })

  it("moves along the mixer with the arrows there, and leaves them to the browser's list", async () => {
    const user = userEvent.setup()
    renderApp()
    act(() => strip("Kick").focus())
    await user.keyboard("{ArrowRight}")
    expect(strip("Clap")).toHaveFocus()

    const selected = useUiStore.getState().selectedTrack
    act(() => screen.getByRole("tree").focus())
    await user.keyboard("{ArrowDown}{ArrowRight}")
    await flush()
    expect(useUiStore.getState().selectedTrack).toBe(selected)
  })
})
