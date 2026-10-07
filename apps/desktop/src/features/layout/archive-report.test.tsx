import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, expect, it } from "vitest"
import { archiveReport, useArchiveStore } from "@/lib/flows/portable"
import { ArchiveReport } from "./archive-report"

afterEach(() =>
  useArchiveStore.setState(useArchiveStore.getInitialState(), true)
)
it("keeps every missing source reviewable in a named dialog until dismissed", async () => {
  const user = userEvent.setup()
  const missing = Array.from(
    { length: 200 },
    (_, i) => `Missing take ${i + 1}.wav`
  ).join("\n")
  archiveReport(missing)
  render(<ArchiveReport />)
  expect(
    screen.getByRole("dialog", { name: "Project archive report" })
  ).toBeVisible()
  expect(screen.getByRole("alert").textContent).toContain(
    "Missing take 200.wav"
  )
  expect(useArchiveStore.getState().report).toBe(missing)
  await user.click(screen.getByRole("button", { name: "Close report" }))
  expect(useArchiveStore.getState().report).toBeNull()
})
