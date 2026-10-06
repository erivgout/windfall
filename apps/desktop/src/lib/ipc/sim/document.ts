import type {
  Command,
  DocumentSnapshot,
  HistoryView,
  Project,
  ProjectPatch,
} from "@/bindings"

import { applyCommand, CommandError, type Applied } from "./commands"
import { buildPatch, emptyTouched, mergeTouched, type Touched } from "./touched"

type Entry = {
  label: string
  before: Project
  after: Project
  touched: Touched
}

/**
 * The one copy of a project plus its undo history. A TypeScript stand-in for
 * `windfall_project::Document`, with the same rules.
 */
export class SimDocument {
  private current: Project
  private entries: Entry[] = []
  private cursor = 0
  private rev = 0
  /** History position the file on disk matches, or null when it matches none. */
  private savedCursor: number | null = 0
  private lastGesture: number | null = null

  constructor(project: Project) {
    this.current = project
  }

  project(): Project {
    return this.current
  }

  revision(): number {
    return this.rev
  }

  /**
   * Applies a command and records it for undo. Consecutive dispatches that
   * carry the same gesture id collapse into one undo step.
   */
  dispatch(command: Command, gesture?: number | null): Applied {
    const applied = applyCommand(this.current, command)
    const top = this.entries[this.cursor - 1]
    const merges =
      gesture != null &&
      gesture === this.lastGesture &&
      this.cursor === this.entries.length &&
      top !== undefined

    if (merges) {
      top.after = applied.project
      top.touched = mergeTouched(top.touched, applied.touched)
      if (this.savedCursor === this.cursor) this.savedCursor = null
    } else {
      if (this.savedCursor !== null && this.savedCursor > this.cursor) {
        this.savedCursor = null
      }
      this.entries = this.entries.slice(0, this.cursor)
      this.entries.push({
        label: applied.label,
        before: this.current,
        after: applied.project,
        touched: applied.touched,
      })
      this.cursor = this.entries.length
    }

    this.current = applied.project
    this.lastGesture = gesture ?? null
    return applied
  }

  undo(): Touched | null {
    const entry = this.entries[this.cursor - 1]
    if (!entry) return null
    this.cursor -= 1
    this.restore(entry.before)
    return entry.touched
  }

  redo(): Touched | null {
    const entry = this.entries[this.cursor]
    if (!entry) return null
    this.cursor += 1
    this.restore(entry.after)
    return entry.touched
  }

  /** Undoes or redoes until `cursor` entries are applied. */
  jump(cursor: number): Touched {
    if (
      !Number.isInteger(cursor) ||
      cursor < 0 ||
      cursor > this.entries.length
    ) {
      throw new CommandError(`History has no step ${cursor}`)
    }
    let touched = emptyTouched()
    while (this.cursor > cursor) {
      const step = this.undo()
      if (step) touched = mergeTouched(touched, step)
    }
    while (this.cursor < cursor) {
      const step = this.redo()
      if (step) touched = mergeTouched(touched, step)
    }
    return touched
  }

  history(): HistoryView {
    return {
      entries: this.entries.map((entry) => ({ label: entry.label })),
      cursor: this.cursor,
    }
  }

  isDirty(): boolean {
    return this.savedCursor !== this.cursor
  }

  markSaved() {
    this.savedCursor = this.cursor
    // A drag that continues after a save must not fold into the saved step.
    this.lastGesture = null
  }

  /** Builds the patch for the UI and bumps the revision. */
  patch(touched: Touched): ProjectPatch {
    this.rev += 1
    return buildPatch(
      this.current,
      touched,
      this.rev,
      this.history(),
      this.isDirty()
    )
  }

  snapshot(path: string | null): DocumentSnapshot {
    return {
      revision: this.rev,
      project: this.current,
      history: this.history(),
      dirty: this.isDirty(),
      path,
    }
  }

  // Ids are never reused, so undo brings back everything except the id
  // counter: that only ever moves forward.
  private restore(project: Project) {
    const nextId = Math.max(project.nextId, this.current.nextId)
    this.current = nextId === project.nextId ? project : { ...project, nextId }
    this.lastGesture = null
  }
}
