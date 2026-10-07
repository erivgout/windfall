import { toast } from "sonner"
import { create } from "zustand"

import { reportError } from "@/lib/errors"
import { backend } from "@/lib/ipc"
import {
  loadSnapshot,
  setProjectPath,
  useProjectStore,
} from "@/lib/store/project"
import { askConfirm } from "@/lib/store/prompts"
import { projectDisplayName } from "@/lib/store/selectors"
import { clearWarnings, samplesReloaded } from "@/lib/store/warnings"
import { fileName } from "@/lib/time"

/** Recently saved or opened project files, newest first. */
export const useRecentStore = create<{ paths: string[] }>(() => ({ paths: [] }))

export async function refreshRecentProjects(): Promise<void> {
  try {
    useRecentStore.setState({ paths: await backend.recentProjects() })
  } catch (error) {
    reportError(error, "Could not list recent projects")
  }
}

/** The project's name as it is shown: its file's name once it has a file. */
export function projectName(): string {
  return projectDisplayName(useProjectStore.getState())
}

async function saveTo(path?: string): Promise<boolean> {
  try {
    const saved = await backend.projectSave(path)
    setProjectPath(saved)
    toast.success("Saved", { description: fileName(saved) })
    void refreshRecentProjects()
    return true
  } catch (error) {
    reportError(error, "Could not save the project")
    return false
  }
}

/** Asks where to save, then saves. Resolves to false when cancelled or failed. */
export async function saveProjectAs(): Promise<boolean> {
  try {
    const path = await backend.pickProjectSavePath(projectName())
    return path === null ? false : await saveTo(path)
  } catch (error) {
    reportError(error, "Could not save the project")
    return false
  }
}

/**
 * Saves to the project's file, or asks for one when it has none: a project
 * never saved, or a backup, which opens as a copy without a path.
 */
export function saveProject(): Promise<boolean> {
  return useProjectStore.getState().path === null ? saveProjectAs() : saveTo()
}

/**
 * Call before anything that replaces or closes the project. With unsaved
 * edits it asks what to do; it resolves to true when it is safe to go on.
 */
export async function confirmDiscardChanges(): Promise<boolean> {
  if (!useProjectStore.getState().dirty) return true
  const choice = await askConfirm({
    title: `Save changes to "${projectName()}"?`,
    description: "Changes you have not saved will be lost.",
    choices: [
      { id: "discard", label: "Don't save", variant: "destructive" },
      { id: "save", label: "Save" },
    ],
  })
  if (choice === "discard") return true
  if (choice === "save") return saveProject()
  return false
}

export async function newProject(): Promise<void> {
  if (!(await confirmDiscardChanges())) return
  try {
    loadSnapshot(await backend.projectNew())
  } catch (error) {
    reportError(error, "Could not start a new project")
  }
}

export async function openProjectPath(path: string): Promise<void> {
  if (!(await confirmDiscardChanges())) return
  try {
    loadSnapshot(await backend.projectOpen(path))
    void refreshRecentProjects()
  } catch (error) {
    reportError(error, "Could not open the project")
  }
}

export async function openProject(): Promise<void> {
  if (!(await confirmDiscardChanges())) return
  try {
    const path = await backend.pickProjectToOpen()
    if (path === null) return
    loadSnapshot(await backend.projectOpen(path))
    void refreshRecentProjects()
  } catch (error) {
    reportError(error, "Could not open the project")
  }
}

/**
 * Asks the backend to look for the missing sample files again, for after
 * the user has put them back. What is still missing is reported the usual
 * way, as project warnings.
 */
export async function reloadMissingSamples(): Promise<void> {
  clearWarnings()
  let missing: number
  try {
    missing = await backend.samplesReload()
  } catch (error) {
    reportError(error, "Could not reload the samples")
    return
  }
  samplesReloaded()
  if (missing === 0) toast.success("All samples loaded")
  else if (missing === 1) toast.error("1 sample is still missing")
  else toast.error(`${missing} samples are still missing`)
}
