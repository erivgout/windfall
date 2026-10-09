import { act, render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { runAction } from "@/lib/actions"
import { dispatch, historyJump, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"
import { settle, startTestApp } from "@/test/harness"

import { groupRuns, HistoryList } from "./history-list"
import { HistoryPopover } from "./history-popover"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

let stop: () => void

beforeEach(async () => {
  ;({ stop } = await startTestApp())
})
afterEach(() => {
  stop()
  vi.restoreAllMocks()
})

const history = () => useProjectStore.getState().history
const list = () => screen.getByRole("list", { name: "Undo history" })

/** A new channel, then `count` steps set on it, then a new pattern. */
async function makeHistory(count: number) {
  const pattern = useProjectStore.getState().project.patterns[0].id
  const added = await dispatch({ type: "addChannel" })
  if (!added) throw new Error("could not add a channel")
  for (let step = 0; step < count; step += 1) {
    await dispatch({
      type: "toggleStep",
      pattern,
      channel: added.created[0],
      step,
    })
  }
  await dispatch({ type: "addPattern" })
}

describe("groupRuns", () => {
  it("folds steps in a row with the same label into one run", () => {
    const entries = ["Add", "Toggle", "Toggle", "Toggle", "Add", "Toggle"].map(
      (label) => ({ label })
    )
    expect(groupRuns(entries)).toEqual([
      { label: "Add", first: 1, last: 1 },
      { label: "Toggle", first: 2, last: 4 },
      { label: "Add", first: 5, last: 5 },
      { label: "Toggle", first: 6, last: 6 },
    ])
    expect(groupRuns([])).toEqual([])
  })
})

describe("HistoryList", () => {
  it("filters rows while keeping run and individual step jump targets", async () => {
    const user = userEvent.setup()
    await makeHistory(4)
    render(<HistoryList query=" tOgGlE " />)
    expect(screen.queryByRole("button", { name: "Project opened" })).toBeNull()
    expect(screen.queryByRole("button", { name: "Add channel" })).toBeNull()
    expect(screen.queryByRole("button", { name: "Add pattern" })).toBeNull()

    await user.click(screen.getByRole("button", { name: "Toggle step×4" }))
    await settle()
    expect(history().cursor).toBe(5)

    await user.click(
      screen.getByRole("button", { name: "Unfold the 4 steps of Toggle step" })
    )
    const steps = within(
      screen.getByRole("list", { name: "Steps of Toggle step" })
    ).getAllByRole("button")
    await user.click(steps[1])
    await settle()
    expect(history().cursor).toBe(3)
  })

  it("shows the empty message when no rows match and restores blank queries", async () => {
    await makeHistory(4)
    const { rerender } = render(<HistoryList query="missing" />)
    expect(screen.getByText("No steps match.")).toBeVisible()
    expect(screen.queryByRole("list", { name: "Undo history" })).toBeNull()
    expect(history().cursor).toBe(6)

    rerender(<HistoryList query={" \t "} />)
    expect(screen.queryByText("No steps match.")).toBeNull()
    expect(screen.getByRole("button", { name: "Project opened" })).toBeVisible()
    expect(screen.getByRole("button", { name: "Toggle step×4" })).toBeVisible()
  })

  it("shows a run as one row with its count, and jumps to its end", async () => {
    const user = userEvent.setup()
    await makeHistory(16)
    render(<HistoryList />)
    const rows = within(list())
      .getAllByRole("button")
      .map((row) => row.textContent)
    expect(rows).toEqual([
      "Project opened",
      "Add channel",
      "",
      "Toggle step×16",
      "Add pattern",
    ])

    await user.click(screen.getByRole("button", { name: "Toggle step×16" }))
    await settle()
    expect(history().cursor).toBe(17)
    expect(
      screen.getByRole("button", { name: "Toggle step×16" })
    ).toHaveAttribute("aria-current", "step")
  })

  it("unfolds a run into steps that can each be chosen", async () => {
    const user = userEvent.setup()
    await makeHistory(4)
    render(<HistoryList />)
    await user.click(
      screen.getByRole("button", { name: "Unfold the 4 steps of Toggle step" })
    )
    const steps = within(
      screen.getByRole("list", { name: "Steps of Toggle step" })
    ).getAllByRole("button")
    expect(steps).toHaveLength(4)

    await user.click(steps[1])
    await settle()
    expect(history().cursor).toBe(3)
    expect(steps[1]).toHaveAttribute("aria-current", "step")
    // Exactly one row is the current one.
    expect(list().querySelectorAll("[aria-current='step']")).toHaveLength(1)
  })

  it("opens a run by itself when the current step is inside it", async () => {
    await makeHistory(4)
    await historyJump(3)
    render(<HistoryList />)
    const steps = within(
      screen.getByRole("list", { name: "Steps of Toggle step" })
    ).getAllByRole("button")
    expect(steps[1]).toHaveAttribute("aria-current", "step")
  })

  it("scrolls the current step into view when it opens, however far down", async () => {
    const scrolled: [Element, ScrollIntoViewOptions][] = []
    vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(function (
      this: Element,
      options?: boolean | ScrollIntoViewOptions
    ) {
      scrolled.push([this, typeof options === "object" ? options : {}])
    })
    for (let index = 0; index < 20; index += 1) {
      await dispatch({ type: index % 2 ? "addPattern" : "addChannel" })
    }
    render(<HistoryList />)
    const current = within(list()).getAllByRole("button").at(-1)
    expect(current).toHaveAttribute("aria-current", "step")
    expect(scrolled).toEqual([[current, { block: "center" }]])

    // Afterwards the current step is only kept in view.
    await historyJump(5)
    await settle()
    expect(scrolled.at(-1)?.[1]).toEqual({ block: "nearest" })
    expect(scrolled.at(-1)?.[0]).toHaveAttribute("aria-current", "step")
  })
})

describe("HistoryPopover", () => {
  it("filters from its labeled input and clears the filter after closing", async () => {
    const user = userEvent.setup()
    await makeHistory(4)
    render(<HistoryPopover />)
    await user.click(screen.getByRole("button", { name: "Undo history" }))
    const input = screen.getByRole("textbox", { name: "Filter history" })
    await user.type(input, "missing")
    expect(screen.getByText("No steps match.")).toBeVisible()

    await user.keyboard("{Escape}")
    await user.click(screen.getByRole("button", { name: "Undo history" }))
    expect(screen.getByRole("textbox", { name: "Filter history" })).toHaveValue(
      ""
    )
    expect(screen.getByRole("button", { name: "Add channel" })).toBeVisible()

    await user.type(
      screen.getByRole("textbox", { name: "Filter history" }),
      "pattern"
    )
    expect(screen.queryByRole("button", { name: "Add channel" })).toBeNull()
    expect(screen.getByRole("button", { name: "Add pattern" })).toBeVisible()
    act(() => useUiStore.getState().setHistoryOpen(false))
    await runAction("edit.history")
    expect(
      await screen.findByRole("textbox", { name: "Filter history" })
    ).toHaveValue("")
    expect(screen.getByRole("button", { name: "Add channel" })).toBeVisible()
  })

  it("opens from its button and from Edit > History", async () => {
    const user = userEvent.setup()
    render(<HistoryPopover />)
    expect(screen.queryByRole("list", { name: "Undo history" })).toBeNull()

    await runAction("edit.history")
    expect(
      await screen.findByRole("list", { name: "Undo history" })
    ).toBeVisible()
    expect(useUiStore.getState().historyOpen).toBe(true)

    await user.keyboard("{Escape}")
    expect(useUiStore.getState().historyOpen).toBe(false)
    await user.click(screen.getByRole("button", { name: "Undo history" }))
    expect(useUiStore.getState().historyOpen).toBe(true)
  })
})
