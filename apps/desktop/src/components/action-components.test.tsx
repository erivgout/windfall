import { act, fireEvent, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { TooltipProvider } from "@/components/ui/tooltip"
import { registry, useActions, useAppState, type Action } from "@/lib/actions"
import { useUiStore } from "@/lib/store/ui"
import { startTestApp } from "@/test/harness"

import { ActionButton } from "./action-button"
import { ActionMenuItem } from "./action-menu-item"
import { ContextActions, type ContextItem } from "./context-actions"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let snap = false
let canZoom = true
const ran: string[] = []

const TEST_ACTIONS: Action[] = [
  {
    id: "test.snap",
    title: "Snap to the grid",
    section: "Test",
    scope: "pianoRoll",
    defaultShortcut: "G",
    checked: () => snap,
    run: () => {
      snap = !snap
      registry.invalidate()
    },
  },
  {
    id: "test.zoom",
    title: "Zoom to fit…",
    section: "Test",
    scope: "pianoRoll",
    defaultShortcut: "Mod+0",
    enabled: () => canZoom,
    run: () => void ran.push("zoom"),
  },
]

let stop: () => void
let unregister: () => void

beforeEach(async () => {
  snap = false
  canZoom = true
  ran.length = 0
  ;({ stop } = await startTestApp())
  unregister = registry.register(TEST_ACTIONS)
})
afterEach(() => {
  unregister()
  stop()
})

describe("ActionButton", () => {
  const renderButtons = () =>
    render(
      <TooltipProvider>
        <ActionButton action="test.zoom" />
        <ActionButton action="test.zoom">Fit</ActionButton>
        <ActionButton action="test.zoom">{16}</ActionButton>
        <ActionButton action="test.zoom">
          <svg data-testid="icon" />
        </ActionButton>
      </TooltipProvider>
    )

  it("is named by its visible text, and by the action only when it shows an icon", () => {
    renderButtons()
    const buttons = screen.getAllByRole("button")
    expect(buttons.map((button) => button.getAttribute("aria-label"))).toEqual([
      null,
      null,
      null,
      "Zoom to fit",
    ])
    expect(screen.getByRole("button", { name: "Fit" })).toBeVisible()
    expect(screen.getByRole("button", { name: "16" })).toBeVisible()
    // The title alone and the icon are both the action's name.
    expect(screen.getAllByRole("button", { name: "Zoom to fit" })).toHaveLength(
      2
    )
  })

  it("follows state the action reads from a panel's own store", async () => {
    renderButtons()
    const button = screen.getByRole("button", { name: "Fit" })
    expect(button).toBeEnabled()
    canZoom = false
    act(() => registry.invalidate())
    expect(button).toBeDisabled()
    canZoom = true
    act(() => registry.invalidate())
    await userEvent.setup().click(button)
    expect(ran).toEqual(["zoom"])
  })
})

function Menu({ ids }: { ids: string[] }) {
  const actions = useActions()
  const state = useAppState()
  return (
    <DropdownMenu open>
      <DropdownMenuTrigger>Menu</DropdownMenuTrigger>
      <DropdownMenuContent>
        {ids.map((id) => {
          const action = actions.find((item) => item.id === id)
          return (
            action && <ActionMenuItem key={id} action={action} state={state} />
          )
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

describe("ActionMenuItem", () => {
  it("shows an on/off action as a checkbox item that follows its state", async () => {
    const user = userEvent.setup()
    render(<Menu ids={["test.snap", "test.zoom"]} />)
    const item = await screen.findByRole("menuitemcheckbox", {
      name: /^Snap to the grid/,
    })
    expect(item).toHaveAttribute("aria-checked", "false")
    expect(within(item).getByText("G")).toBeVisible()
    expect(screen.getByRole("menuitem", { name: /^Zoom to fit/ })).toBeVisible()

    await user.click(item)
    expect(snap).toBe(true)
  })

  it("is ticked while the action is on, and greyed while it cannot run", async () => {
    snap = true
    canZoom = false
    render(<Menu ids={["test.snap", "test.zoom"]} />)
    expect(
      await screen.findByRole("menuitemcheckbox", { name: /^Snap/ })
    ).toHaveAttribute("aria-checked", "true")
    expect(screen.getByRole("menuitem", { name: /^Zoom/ })).toHaveAttribute(
      "aria-disabled",
      "true"
    )
  })
})

describe("ContextActions", () => {
  function open(items: ContextItem[]) {
    render(
      <ContextActions items={items}>
        <div data-testid="area">Area</div>
      </ContextActions>
    )
    fireEvent.contextMenu(screen.getByTestId("area"))
    return screen.findByRole("menu")
  }

  it("ticks an action that is on, and leaves room for the tick on the others", async () => {
    snap = true
    const menu = await open(["test.snap", "test.zoom"])
    const toggle = within(menu).getByRole("menuitemcheckbox", {
      name: /^Snap to the grid/,
    })
    expect(toggle).toHaveAttribute("aria-checked", "true")
    expect(toggle.querySelector("svg")).not.toBeNull()
    const plain = within(menu).getByRole("menuitem", { name: /^Zoom to fit/ })
    expect(plain).not.toHaveAttribute("aria-checked")
    // An empty slot of the tick's width keeps the titles in one column.
    expect(plain.querySelector("[aria-hidden] svg")).toBeNull()
    expect(plain.querySelector("[aria-hidden]")).not.toBeNull()
  })

  it("follows a toggle that changes while the menu is open", async () => {
    const menu = await open(["test.snap"])
    const toggle = () => within(menu).getByRole("menuitemcheckbox")
    expect(toggle()).toHaveAttribute("aria-checked", "false")
    snap = true
    act(() => registry.invalidate())
    expect(toggle()).toHaveAttribute("aria-checked", "true")
  })

  it("ticks an inline entry that says it is on", async () => {
    const menu = await open([
      { title: "Loop", checked: true, run() {} },
      { title: "Follow", checked: false, run() {} },
      { title: "Rename", run() {} },
    ])
    expect(
      within(menu).getByRole("menuitemcheckbox", { name: "Loop" })
    ).toHaveAttribute("aria-checked", "true")
    expect(
      within(menu).getByRole("menuitemcheckbox", { name: "Follow" })
    ).toHaveAttribute("aria-checked", "false")
    expect(within(menu).getByRole("menuitem", { name: "Rename" })).toBeVisible()
  })

  it("shows the key an action has in its own panel, in each preset", async () => {
    const remove = registry.register(
      [
        {
          id: "test.draw",
          title: "Draw",
          section: "Test",
          scope: "playlist",
          defaultShortcut: "D",
          run() {},
        },
      ],
      { presets: { fl: { "test.draw": ["P"] } } }
    )
    const menu = await open(["test.draw", "test.zoom"])
    const draw = () => within(menu).getByRole("menuitem", { name: /^Draw/ })
    expect(within(draw()).getByText("D")).toBeVisible()
    expect(
      within(within(menu).getByRole("menuitem", { name: /^Zoom/ })).getByText(
        "Ctrl+0"
      )
    ).toBeVisible()

    act(() => useUiStore.getState().setKeymap("fl"))
    expect(within(draw()).getByText("P")).toBeVisible()
    remove()
  })
})
