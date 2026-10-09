import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"

import type { ProjectSettings } from "@/bindings"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { announceProjectReplaced } from "@/lib/store/replaced"

import { ProjectInfoControls } from "./index"
import { ProjectInfoPanel } from "./panel"

vi.mock("@/lib/store/project", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/lib/store/project")>()),
  dispatch: vi.fn(),
}))

const settings: ProjectSettings = {
  name: "Song",
  tempoBpm: 120,
  timeSignature: { numerator: 4, denominator: 4 },
  swing: 0,
}

afterEach(() => vi.resetAllMocks())

describe("project info", () => {
  it("shows omitted fields as empty and sends one trimmed patch for all edits", async () => {
    const project = useProjectStore.getState().project
    useProjectStore.setState({ project: { ...project, settings } })
    vi.mocked(dispatch).mockResolvedValue(null)
    render(<ProjectInfoControls />)
    fireEvent.click(screen.getByRole("button", { name: "Project info" }))
    const author = await screen.findByLabelText("Author")
    expect(screen.getByLabelText("Name")).toHaveValue("Song")
    expect(author).toHaveValue("")
    expect(screen.getByLabelText("Genre")).toHaveValue("")
    expect(screen.getByLabelText("Comments")).toHaveValue("")
    expect(
      screen.getByRole("button", { name: "Save project info" })
    ).toBeDisabled()
    fireEvent.change(author, { target: { value: "  Artist  " } })
    fireEvent.change(screen.getByLabelText("Genre"), {
      target: { value: "  Ambient " },
    })
    fireEvent.change(screen.getByLabelText("Comments"), {
      target: { value: "  First line\nSecond line  " },
    })
    expect(dispatch).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole("button", { name: "Save project info" }))
    await waitFor(() => expect(dispatch).toHaveBeenCalledTimes(1))
    expect(dispatch).toHaveBeenCalledWith({
      type: "updateSettings",
      patch: {
        author: "Artist",
        genre: "Ambient",
        comments: "First line\nSecond line",
      },
    })
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not save project info."
    )
  })

  it("sends a trimmed name with the other edits in one settings patch", async () => {
    const project = useProjectStore.getState().project
    useProjectStore.setState({ project: { ...project, settings } })
    vi.mocked(dispatch).mockResolvedValue(null)
    render(<ProjectInfoControls />)
    fireEvent.click(screen.getByRole("button", { name: "Project info" }))
    fireEvent.change(await screen.findByLabelText("Name"), {
      target: { value: "  New song  " },
    })
    fireEvent.change(screen.getByLabelText("Author"), {
      target: { value: "  Artist  " },
    })
    fireEvent.change(screen.getByLabelText("Genre"), {
      target: { value: " Ambient " },
    })
    fireEvent.change(screen.getByLabelText("Comments"), {
      target: { value: " Notes " },
    })
    fireEvent.click(screen.getByRole("button", { name: "Save project info" }))
    await waitFor(() => expect(dispatch).toHaveBeenCalledTimes(1))
    expect(dispatch).toHaveBeenCalledWith({
      type: "updateSettings",
      patch: {
        name: "New song",
        author: "Artist",
        genre: "Ambient",
        comments: "Notes",
      },
    })
  })

  it("saves a name-only edit without unchanged metadata", async () => {
    const onSave = vi.fn(async () => true)
    render(
      <ProjectInfoPanel
        settings={{
          ...settings,
          author: "Artist",
          genre: "Ambient",
          comments: "Notes",
        }}
        onSave={onSave}
      />
    )
    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "New song" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Save project info" }))
    await waitFor(() =>
      expect(onSave).toHaveBeenCalledWith({ name: "New song" })
    )
  })

  it("omits a name unchanged after trimming when saving another field", async () => {
    const onSave = vi.fn(async () => true)
    render(<ProjectInfoPanel settings={settings} onSave={onSave} />)
    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "  Song  " },
    })
    expect(
      screen.getByRole("button", { name: "Save project info" })
    ).toBeDisabled()
    fireEvent.change(screen.getByLabelText("Comments"), {
      target: { value: "Notes" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Save project info" }))
    await waitFor(() =>
      expect(onSave).toHaveBeenCalledWith({ comments: "Notes" })
    )
  })

  it.each([
    [" \n ", "Name must not be blank."],
    ["é".repeat(128) + "x", "Name must be at most 256 bytes."],
  ])("blocks all edits for an invalid name %j", (name, error) => {
    const onSave = vi.fn(async () => true)
    render(<ProjectInfoPanel settings={settings} onSave={onSave} />)
    fireEvent.change(screen.getByLabelText("Name"), { target: { value: name } })
    fireEvent.change(screen.getByLabelText("Author"), {
      target: { value: "Artist" },
    })
    expect(screen.getByLabelText("Name")).toHaveAttribute(
      "aria-invalid",
      "true"
    )
    expect(screen.getByRole("alert")).toHaveTextContent(error)
    expect(
      screen.getByRole("button", { name: "Save project info" })
    ).toBeDisabled()
    fireEvent.submit(screen.getByRole("form", { name: "Project info" }))
    expect(onSave).not.toHaveBeenCalled()
  })

  it("disables controls during a save and refuses duplicate submissions", async () => {
    let finish: (value: boolean) => void = () => {}
    const onSave = vi.fn(
      () =>
        new Promise<boolean>((resolve) => {
          finish = resolve
        })
    )
    render(
      <ProjectInfoPanel
        settings={{ ...settings, author: "Artist" }}
        onSave={onSave}
      />
    )
    fireEvent.change(screen.getByLabelText("Comments"), {
      target: { value: "Notes" },
    })
    fireEvent.submit(screen.getByRole("form", { name: "Project info" }))
    for (const label of ["Name", "Author", "Genre", "Comments"]) {
      expect(screen.getByLabelText(label)).toBeDisabled()
    }
    expect(screen.getByRole("button", { name: "Saving…" })).toBeDisabled()
    fireEvent.submit(screen.getByRole("form", { name: "Project info" }))
    expect(onSave).toHaveBeenCalledTimes(1)
    expect(onSave).toHaveBeenCalledWith({ comments: "Notes" })
    await act(async () => finish(true))
    expect(screen.getByLabelText("Comments")).toBeEnabled()
  })

  it.each([
    ["Author", 256],
    ["Genre", 128],
    ["Comments", 16_384],
  ])("validates the %s cap in trimmed UTF-8 bytes", (label, cap) => {
    const onSave = vi.fn(async () => true)
    render(<ProjectInfoPanel settings={settings} onSave={onSave} />)
    fireEvent.change(screen.getByLabelText(label), {
      target: { value: ` ${"é".repeat(cap / 2)} ` },
    })
    expect(
      screen.getByRole("button", { name: "Save project info" })
    ).toBeEnabled()
    fireEvent.change(screen.getByLabelText(label), {
      target: { value: "é".repeat(cap / 2) + "x" },
    })
    expect(screen.getByLabelText(label)).toHaveAttribute("aria-invalid", "true")
    expect(
      screen.getByRole("button", { name: "Save project info" })
    ).toBeDisabled()
    fireEvent.submit(screen.getByRole("form", { name: "Project info" }))
    expect(onSave).not.toHaveBeenCalled()
  })

  it("sends an empty string to clear a field and keeps unrelated fields out", async () => {
    const onSave = vi.fn(async () => true)
    render(
      <ProjectInfoPanel
        settings={{ ...settings, author: "Artist", comments: "Notes" }}
        onSave={onSave}
      />
    )
    fireEvent.change(screen.getByLabelText("Comments"), {
      target: { value: " \n " },
    })
    fireEvent.click(screen.getByRole("button", { name: "Save project info" }))
    await waitFor(() => expect(onSave).toHaveBeenCalledWith({ comments: "" }))
  })

  it("discards a draft when another project replaces the current one", async () => {
    render(<ProjectInfoControls />)
    fireEvent.click(screen.getByRole("button", { name: "Project info" }))
    fireEvent.change(await screen.findByLabelText("Author"), {
      target: { value: "Unsaved draft" },
    })
    act(() => announceProjectReplaced())
    expect(screen.getByLabelText("Author")).toHaveValue("")
    expect(dispatch).not.toHaveBeenCalled()
  })

  it("refreshes the name draft when the document name changes", async () => {
    const project = useProjectStore.getState().project
    useProjectStore.setState({ project: { ...project, settings } })
    render(<ProjectInfoControls />)
    fireEvent.click(screen.getByRole("button", { name: "Project info" }))
    fireEvent.change(await screen.findByLabelText("Name"), {
      target: { value: "Unsaved name" },
    })
    act(() => {
      useProjectStore.setState({
        project: {
          ...project,
          settings: { ...settings, name: "Restored song" },
        },
      })
    })
    expect(screen.getByLabelText("Name")).toHaveValue("Restored song")
    expect(dispatch).not.toHaveBeenCalled()
  })
})
