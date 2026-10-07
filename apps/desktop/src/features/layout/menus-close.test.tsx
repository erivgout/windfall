import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import type { ReactNode } from "react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { useRackStore } from "@/features/channel-rack/rack-store"
import { useEffectsUi } from "@/features/mixer/effects-ui"
import { useMixerUi } from "@/features/mixer/mixer-ui"
import { usePianoRollStore } from "@/features/piano-roll/store"
import { usePlaylistStore } from "@/features/playlist/store"
import { setBackend } from "@/lib/ipc"
import { createMockBackend, type MockBackend } from "@/lib/ipc/mock"
import { useEngineStore } from "@/lib/store/engine"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore, type CenterTab } from "@/lib/store/ui"
import { useWarningsStore } from "@/lib/store/warnings"
import { settle, TEST_DIALOGS } from "@/test/harness"

import { AppShell } from "./app-shell"

/*
 * Every menu and popover that is opened to pick one thing closes once the
 * thing is picked. The pattern menu did not, and left the window inert
 * until Escape, so this walks all of them in the whole window.
 */

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
  Toaster: () => null,
}))

// jsdom lays nothing out, so the real resize handles would take every click.
vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: ReactNode }) => (
    <div className="flex">{children}</div>
  ),
  ResizablePanel: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => null,
}))

let backend: MockBackend

beforeEach(async () => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null)
  backend = createMockBackend({ storage: null, dialogs: TEST_DIALOGS })
  setBackend(backend)
  for (const store of [
    useProjectStore,
    useTransportStore,
    useEngineStore,
    useUiStore,
    usePromptStore,
    useWarningsStore,
    useEffectsUi,
    useMixerUi,
    useRackStore,
    usePianoRollStore,
    usePlaylistStore,
  ]) {
    ;(store as { setState(state: unknown, replace: true): void }).setState(
      store.getInitialState(),
      true
    )
  }
})
afterEach(() => {
  backend.dispose()
  vi.restoreAllMocks()
})

const flush = () => act(settle)
const project = () => useProjectStore.getState().project

/** Everything that lies over the window: menus, lists and popovers. */
const overlays = () =>
  [
    ...document.querySelectorAll(
      "[role=menu], [role=listbox], [role=dialog], [data-slot=popover-content]"
    ),
  ].filter((element) => !element.closest("[data-closed]"))

const show = (tab: CenterTab) =>
  act(async () => useUiStore.getState().showCenterTab(tab))

type Step = {
  what: string
  tab?: CenterTab
  /** Opens the menu or popover. */
  open(): unknown
  /** What to pick in it. */
  pick(): HTMLElement
  /** Pick with a whole pointer press, as a select needs. */
  press?: boolean
}

const strip = (name: string) =>
  screen.getByRole("group", { name: new RegExp(`^${name}, `) })
const button = (name: string | RegExp) =>
  screen.getAllByRole("button", { name })[0]
/** The entries of the menu opened last: a submenu, when one is open. */
const innermostItems = () => [
  ...screen
    .getAllByRole("menu")
    .at(-1)!
    .querySelectorAll<HTMLElement>(
      "[role=menuitem], [role=menuitemcheckbox], [role=menuitemradio]"
    ),
]
const rightClick = (element: Element) =>
  fireEvent.contextMenu(element, { clientX: 20, clientY: 20 })

describe("menus and popovers", () => {
  it(
    "close once something is picked in them",
    { timeout: 60_000 },
    async () => {
      const user = userEvent.setup()
      render(<AppShell />)
      await flush()
      // A synth, an effect and a send, so every menu has something to open on.
      let synth = 0
      await act(async () => {
        const added = await dispatch({
          type: "addChannel",
          instrument: "subtractiveSynth",
        })
        synth = added!.created[0]
        const [master, kick, clap] = project().mixer.tracks
        await dispatch({ type: "addEffect", track: kick.id, kind: "reverb" })
        await dispatch({ type: "setSend", from: kick.id, to: clap.id, gain: 1 })
        await backend.addAudioClipFromFile("/factory/Loops/Drum loop 128.wav", {
          start: 0,
          mixerTrack: master.id,
        })
        useUiStore.getState().selectTrack(kick.id)
        useEffectsUi.getState().setInspectorOpen(true)
        useRackStore.getState().setInspectorOpen(true)
        useUiStore.getState().selectChannel(synth)
        await settle()
      })

      const steps: Step[] = [
        {
          what: "the pattern menu",
          open: () => fireEvent.click(button(/^Pattern:/)),
          pick: () => screen.getByRole("menuitemradio", { name: "Pattern 1" }),
        },
        {
          what: "a channel's routing menu",
          open: () => fireEvent.click(button(/^Kick plays into Kick/)),
          pick: () => screen.getAllByRole("menuitemradio")[1],
        },
        {
          what: "a channel's colors",
          open: () => fireEvent.click(button("Kick color")),
          pick: () => screen.getByRole("button", { name: "Teal" }),
        },
        {
          what: "the synth's sounds",
          open: async () => {
            // The steps before left another channel selected.
            await act(async () => useUiStore.getState().selectChannel(synth))
            fireEvent.click(button(/^Sound:/))
          },
          pick: () => screen.getAllByRole("menuitemcheckbox")[1],
        },
        {
          what: "the rack's menu, in a submenu",
          open: async () => {
            rightClick(screen.getByRole("group", { name: "Channels" }))
            await flush()
            fireEvent.click(
              screen.getByRole("menuitem", { name: "Pattern length" })
            )
          },
          pick: () => innermostItems()[0],
        },
        {
          what: "a strip's add effect menu",
          open: () =>
            fireEvent.click(
              within(strip("Clap")).getByRole("button", { name: "Add effect" })
            ),
          pick: () => screen.getByRole("menuitem", { name: /^Delay/ }),
        },
        {
          what: "an effect's own menu",
          open: () => fireEvent.click(button(/ actions$/)),
          pick: () => screen.getByRole("menuitem", { name: /^Duplicate/ }),
        },
        {
          what: "a track's output",
          open: () =>
            fireEvent.click(
              within(strip("Hat")).getByRole("button", { name: /^Output/ })
            ),
          pick: () => screen.getAllByRole("menuitemradio")[1],
        },
        {
          what: "a track's add send menu",
          open: () =>
            fireEvent.click(
              within(strip("Hat")).getByRole("button", { name: "Add send" })
            ),
          pick: () => innermostItems()[0],
        },
        {
          what: "a track's colors",
          open: () =>
            fireEvent.click(
              within(strip("Snare")).getByRole("button", {
                name: "Change track color",
              })
            ),
          pick: () => screen.getByRole("button", { name: "Teal" }),
        },
        {
          what: "a strip's menu, in a submenu",
          open: async () => {
            rightClick(
              within(strip("Snare")).getByText("Snare", {
                selector: "[data-slot=strip-name]",
              })
            )
            await flush()
            fireEvent.click(
              screen.getByRole("menuitem", { name: "Add effect" })
            )
          },
          pick: () => innermostItems()[0],
        },
        {
          what: "a knob's menu",
          open: () => rightClick(screen.getAllByRole("slider")[0]),
          pick: () => screen.getByRole("menuitem", { name: /^Copy value/ }),
        },
        {
          what: "the playlist's snap",
          tab: "playlist",
          open: () => fireEvent.click(button(/^Snap:/)),
          pick: () => screen.getAllByRole("menuitemradio")[0],
        },
        {
          what: "an audio clip's mixer track",
          tab: "playlist",
          open: async () => {
            await act(async () => {
              const clip = project().playlist.clips[0]
              usePlaylistStore.getState().select([clip.id])
            })
            fireEvent.click(button(/^Mixer track:/))
          },
          pick: () => screen.getByRole("menuitem", { name: "Kick" }),
        },
        {
          what: "the piano roll's lane",
          tab: "pianoRoll",
          open: () => fireEvent.click(button(/^Lane shows/)),
          pick: () => screen.getAllByRole("menuitemradio")[1],
        },
        {
          what: "the piano roll's snap",
          tab: "pianoRoll",
          open: () =>
            fireEvent.click(screen.getByRole("combobox", { name: "Snap" })),
          pick: () => screen.getAllByRole("option")[1],
          press: true,
        },
      ]

      for (const step of steps) {
        await show(step.tab ?? "channelRack")
        await flush()
        expect(overlays(), `before ${step.what}`).toHaveLength(0)
        await step.open()
        await flush()
        expect(overlays().length, `${step.what} opens`).toBeGreaterThan(0)
        // A select takes a pick only from a whole press and release.
        if (step.press) await user.click(step.pick())
        else fireEvent.click(step.pick())
        await flush()
        await waitFor(() =>
          expect(overlays(), `${step.what} is still open`).toHaveLength(0)
        )
      }
    }
  )
})
