import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, expect, it } from "vitest"

import { registry } from "@/lib/actions"

import { closeProjectOverview, registerProjectOverviewActions } from "./actions"
import { ProjectOverviewPanel } from "./panel"

afterEach(() => closeProjectOverview())

it("mounts the overview from the View action and unmounts it on close", () => {
  const unregister = registerProjectOverviewActions()
  try {
    render(<ProjectOverviewPanel />)
    expect(screen.queryByRole("table")).not.toBeInTheDocument()
    const action = registry.get("view.projectOverview")
    expect(action?.title).toBe("Show project overview")
    expect(action?.section).toBe("View")
    act(() => {
      void action?.run()
    })
    expect(
      screen.getByRole("dialog", { name: "Project overview" })
    ).toBeVisible()
    expect(
      screen.getByRole("table", { name: "Channels and patterns" })
    ).toBeVisible()
    fireEvent.click(screen.getByRole("button", { name: "Close" }))
    expect(screen.queryByRole("table")).not.toBeInTheDocument()
    act(() => {
      void action?.run()
    })
    expect(screen.getByRole("table")).toBeVisible()
    act(() => unregister())
    expect(screen.queryByRole("table")).not.toBeInTheDocument()
    expect(registry.get("view.projectOverview")).toBeUndefined()
  } finally {
    unregister()
  }
})
