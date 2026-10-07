import type {
  ChannelId,
  Command,
  DispatchResult,
  Note,
  NoteUpdate,
} from "@/bindings"
import {
  hitTestPoint,
  keyToRow,
  queryRect,
  rowToKey,
  withAlpha,
  xToTick,
  yToRow,
  type Hit,
  type IndexedBatch,
  type Layer,
  type Marquee,
  type Rgba,
  type Viewport,
} from "@/lib/canvas"
import { readClipboard, writeClipboard } from "./clipboard"
import {
  cellsBetween,
  clampKey,
  copyNotes,
  duplicateRight,
  endAfterUpdates,
  endOfInits,
  MAX_KEY,
  moveDelta,
  moveLimits,
  moveUpdates,
  movedCopies,
  paintSpacing,
  pasteInits,
  pasteStart,
  quantizeEnds,
  quantizeStarts,
  resizeDelta,
  resizeUpdates,
  rowsAlongSegment,
  withExtension,
  type MoveDelta,
  type MoveLimits,
  type PasteTarget,
  type PatternInfo,
  type ResizeDelta,
  type ResizeEdge,
} from "./edit-math"
import { pressIntent, type Intent, type PressButton } from "./intents"
import {
  buildScene,
  FALLBACK_PALETTE,
  selectedNotes,
  type Scene,
} from "./scene"
import { snapFloor } from "./snap"
import type { Tool } from "./store"

/** A pointer position in CSS pixels inside the grid, with the modifiers held. */
export type PointerInput = {
  x: number
  y: number
  shift: boolean
  ctrl: boolean
  alt: boolean
}

/** The part of the canvas view the editor drives. `TimeGridView` is one. */
export type EditorSurface = {
  readonly viewport: Viewport
  setItems(items: IndexedBatch | null): void
  setDragOffset(ticks: number, rows: number): void
  setDragResize(startTicks: number, endTicks: number, minLength?: number): void
  setMarquee(marquee: Marquee | null): void
  invalidate(layer?: Layer): void
}

export type EditorContext = {
  pattern: PatternInfo
  channel: ChannelId
  /** The lane's notes, sorted by start. The same array until they change. */
  notes: readonly Note[]
}

export type EditorSettings = {
  tool: Tool
  /** Snap interval in ticks. 0 is off. */
  snap: number
  lastLength: number
  lastVelocity: number
}

/** Everything the editor needs from outside, so tests can stand in for it. */
export type EditorHost = {
  dispatch(command: Command): Promise<DispatchResult | null>
  /** What is being edited, read from the project right now. */
  context(): EditorContext | null
  settings(): EditorSettings
  remember(length: number, velocity: number): void
  noteOn(channel: ChannelId, key: number, velocity: number): void
  noteOff(channel: ChannelId, key: number): void
}

/** How a drag in progress shows the selected notes, before it is committed. */
export type DragPreview = {
  ticks: number
  keys: number
  resize: ResizeDelta | null
  /** The drop will copy the notes and leave the originals. */
  duplicate: boolean
}

export type Hover = {
  tick: number
  key: number
  intent: Intent | null
}

/**
 * - "scene": the notes on screen changed.
 * - "selection": which notes are selected changed.
 * - "drag": the preview of a drag in progress changed.
 * - "hover": what is under the pointer changed.
 */
export type EditorEvent = "scene" | "selection" | "drag" | "hover"

type Press = { x: number; y: number }

type Gesture =
  | { kind: "idle" }
  | {
      kind: "marquee"
      press: Press
      tick: number
      row: number
      base: ReadonlySet<number>
      hitId: number | null
      shift: boolean
      moved: boolean
    }
  | {
      kind: "move"
      press: Press
      tick: number
      row: number
      grabbed: Note
      notes: Note[]
      limits: MoveLimits
      delta: MoveDelta
      wasSelected: boolean
      shift: boolean
      /** The note is being drawn and is not in the project yet. */
      provisional: boolean
      moved: boolean
    }
  | {
      kind: "resize"
      press: Press
      tick: number
      edge: ResizeEdge
      grabbed: Note
      notes: Note[]
      delta: ResizeDelta
      moved: boolean
    }
  | {
      kind: "paint"
      origin: number
      spacing: number
      length: number
      velocity: number
      cells: Map<number, Note>
      lastCell: number
    }
  | { kind: "erase"; ids: Set<number>; tick: number; row: number }

const IDLE: Gesture = { kind: "idle" }
/** A press has to travel this far before it counts as a drag. */
const DEAD_ZONE_PX = 3
/** Ids of notes that are drawn but not in the project yet count down from here. */
const PROVISIONAL_ID = -1000
const ERASED_ALPHA = 0.22

function noSnap(input: PointerInput): boolean {
  return input.shift || input.alt
}

/**
 * The piano roll without its canvas: the notes on screen, the selection,
 * the pointer gestures of each tool and the edits they turn into. Every
 * drag shows a preview and dispatches one command when it ends.
 */
export class Editor {
  hover: Hover | null = null

  private readonly host: EditorHost
  private surface: EditorSurface | null = null
  private ctx: EditorContext | null = null
  private palette: readonly Rgba[] = FALLBACK_PALETTE
  private scene: Scene = buildScene([], [], FALLBACK_PALETTE, new Set())
  private selected = new Set<number>()
  private provisional: Note[] = []
  private gesture: Gesture = IDLE
  private preview: DragPreview | null = null
  private sounding: number | null = null
  private listeners = new Set<(event: EditorEvent) => void>()

  constructor(host: EditorHost) {
    this.host = host
  }

  subscribe(listener: (event: EditorEvent) => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  attach(surface: EditorSurface | null): void {
    this.surface = surface
    surface?.setItems(this.scene.items)
  }

  get context(): EditorContext | null {
    return this.ctx
  }

  get notes(): readonly Note[] {
    return this.scene.notes
  }

  get items(): IndexedBatch {
    return this.scene.items
  }

  get selection(): ReadonlySet<number> {
    return this.selected
  }

  get drag(): DragPreview | null {
    return this.preview
  }

  /** How many notes of the project are selected. */
  get selectionCount(): number {
    let count = 0
    for (const id of this.selected) if (id >= 0) count++
    return count
  }

  get busy(): boolean {
    return this.gesture.kind !== "idle"
  }

  selectedNotes(): Note[] {
    return selectedNotes(this.scene, this.selected)
  }

  /** Colors change with the theme and with the channel. */
  setPalette(palette: readonly Rgba[]): void {
    this.palette = palette.length > 0 ? palette : FALLBACK_PALETTE
    this.rebuild()
  }

  /**
   * Follows the project. Called on every render of the panel and after
   * every commit; it only does work when the lane really changed.
   */
  setContext(next: EditorContext | null): void {
    this.follow(next, false)
  }

  private follow(next: EditorContext | null, force: boolean): void {
    const previous = this.ctx
    this.ctx = next
    if (!next) {
      this.abandon()
      this.selected = new Set()
      this.rebuild()
      this.emit("selection")
      return
    }
    const sameLane =
      previous?.channel === next.channel &&
      previous.pattern.id === next.pattern.id
    if (sameLane && previous.notes === next.notes && !force) return
    this.abandon()
    if (!sameLane) {
      this.selected = new Set()
    } else if (this.selected.size > 0) {
      // Undo or another window can remove notes that were selected.
      const alive = new Set<number>()
      for (const note of next.notes) {
        if (this.selected.has(note.id)) alive.add(note.id)
      }
      this.selected = alive
    }
    this.rebuild()
    this.emit("selection")
  }

  // Pointer gestures

  /** The intent a press at this point would have, for the cursor and hint. */
  pointerHover(input: PointerInput): void {
    const surface = this.surface
    if (!surface || this.gesture.kind !== "idle") return
    const hit = this.hitAt(input)
    const { tool } = this.host.settings()
    this.setHover(
      xToTick(surface.viewport, input.x),
      this.keyAt(input.y),
      pressIntent(tool, "left", hit?.part ?? null, input)
    )
  }

  pointerLeave(): void {
    if (this.gesture.kind !== "idle" || this.hover === null) return
    this.hover = null
    this.emit("hover")
  }

  /** Starts a gesture. Returns what the press does. */
  pointerDown(input: PointerInput, button: PressButton): Intent | null {
    const surface = this.surface
    const ctx = this.ctx
    if (!surface || !ctx) return null
    if (this.gesture.kind !== "idle") this.cancel()
    const settings = this.host.settings()
    const hit = this.hitAt(input)
    const intent = pressIntent(settings.tool, button, hit?.part ?? null, input)
    const tick = xToTick(surface.viewport, input.x)
    const row = yToRow(surface.viewport, input.y)
    const press = { x: input.x, y: input.y }

    switch (intent.kind) {
      case "menu":
        if (hit && !this.selected.has(hit.id)) this.setSelection([hit.id])
        break
      case "marquee":
        this.gesture = {
          kind: "marquee",
          press,
          tick,
          row,
          base: input.shift ? new Set(this.selected) : new Set(),
          hitId: hit?.id ?? null,
          shift: input.shift,
          moved: false,
        }
        break
      case "erase":
        this.gesture = { kind: "erase", ids: new Set(), tick, row }
        if (hit) this.markErased([hit.index])
        this.eraseAlong(tick, row, tick, row)
        break
      case "move":
        if (hit) this.beginMove(hit, input, press, tick, row)
        break
      case "resize":
        if (hit) this.beginResize(hit, intent.edge, input, press, tick)
        break
      case "draw":
        this.beginDraw(input, press, tick, row, settings)
        break
      case "paint":
        this.beginPaint(input, tick, row, settings)
        break
      default: {
        const _exhaustive: never = intent
        return _exhaustive
      }
    }
    return intent
  }

  pointerMove(input: PointerInput): void {
    const surface = this.surface
    const gesture = this.gesture
    if (!surface) return
    const viewport = surface.viewport
    const tick = xToTick(viewport, input.x)
    const row = yToRow(viewport, input.y)

    switch (gesture.kind) {
      case "idle":
        this.pointerHover(input)
        return
      case "marquee": {
        if (!gesture.moved && !pastDeadZone(gesture.press, input)) return
        gesture.moved = true
        surface.setMarquee({
          tick0: gesture.tick,
          row0: gesture.row,
          tick1: tick,
          row1: row,
        })
        const batch = this.scene.items.batch
        const indices = queryRect(
          this.scene.items,
          Math.min(gesture.tick, tick),
          Math.max(gesture.tick, tick),
          Math.min(gesture.row, row),
          Math.max(gesture.row, row)
        )
        const ids = new Set(gesture.base)
        for (const index of indices) ids.add(batch.ids[index])
        this.applySelection(ids)
        this.setHover(tick, this.keyAt(input.y), { kind: "marquee" })
        return
      }
      case "move": {
        if (!gesture.moved && !pastDeadZone(gesture.press, input)) return
        gesture.moved = true
        const snap = noSnap(input) ? 0 : this.host.settings().snap
        const delta = moveDelta(
          tick - gesture.tick,
          Math.floor(gesture.row) - Math.floor(row),
          snap,
          gesture.limits
        )
        gesture.delta = delta
        surface.setDragOffset(delta.ticks, -delta.keys)
        this.preview = {
          ticks: delta.ticks,
          keys: delta.keys,
          resize: null,
          duplicate: input.ctrl && !gesture.provisional,
        }
        const key = gesture.grabbed.key + delta.keys
        this.sound(key, gesture.grabbed.velocity)
        this.setHover(gesture.grabbed.start + delta.ticks, key, {
          kind: gesture.provisional ? "draw" : "move",
        })
        this.emit("drag")
        return
      }
      case "resize": {
        if (!gesture.moved && !pastDeadZone(gesture.press, input)) return
        gesture.moved = true
        const snap = noSnap(input) ? 0 : this.host.settings().snap
        const delta = resizeDelta(
          gesture.edge,
          tick - gesture.tick,
          snap,
          gesture.notes
        )
        gesture.delta = delta
        surface.setDragResize(delta.start, delta.end, delta.minLength)
        this.preview = { ticks: 0, keys: 0, resize: delta, duplicate: false }
        this.emit("drag")
        return
      }
      case "paint": {
        const cell = Math.floor((tick - gesture.origin) / gesture.spacing)
        const cells = cellsBetween(gesture.lastCell, cell)
        gesture.lastCell = cell
        this.paintCells(gesture, cells, row)
        this.setHover(tick, this.keyAt(input.y), { kind: "paint" })
        return
      }
      case "erase":
        this.eraseAlong(gesture.tick, gesture.row, tick, row)
        gesture.tick = tick
        gesture.row = row
        this.setHover(tick, this.keyAt(input.y), { kind: "erase" })
        return
      default: {
        const _exhaustive: never = gesture
        return _exhaustive
      }
    }
  }

  /** Ends the gesture and commits it as one command. */
  pointerUp(input: PointerInput): void {
    const gesture = this.gesture
    const ctx = this.ctx
    if (gesture.kind === "idle") return
    this.pointerMove(input)
    this.gesture = IDLE
    this.sound(null)
    this.surface?.setMarquee(null)
    if (!ctx) {
      this.abandon()
      return
    }

    switch (gesture.kind) {
      case "marquee":
        if (gesture.moved) break
        if (gesture.hitId === null) {
          if (!gesture.shift) this.setSelection([])
        } else if (gesture.shift) {
          this.toggleSelected(gesture.hitId)
        } else {
          this.setSelection([gesture.hitId])
        }
        break
      case "move":
        this.finishMove(gesture, input, ctx)
        break
      case "resize": {
        const updates = resizeUpdates(gesture.notes, gesture.delta)
        if (updates.length === 0) {
          this.clearPreview()
          break
        }
        const grabbed = updates.find(
          (update) => update.id === gesture.grabbed.id
        )
        this.host.remember(
          grabbed?.patch.length ?? gesture.grabbed.length,
          gesture.grabbed.velocity
        )
        this.commitDrag(
          withExtension(
            {
              type: "updateNotes",
              pattern: ctx.pattern.id,
              channel: ctx.channel,
              updates,
            },
            "Resize notes",
            ctx.pattern,
            endAfterUpdates(gesture.notes, updates)
          )
        )
        break
      }
      case "paint": {
        const painted = [...gesture.cells.values()].sort(
          (a, b) => a.start - b.start
        )
        if (painted.length === 0) break
        const notes = movedCopies(painted, { ticks: 0, keys: 0 })
        this.commitDrag(
          withExtension(
            {
              type: "addNotes",
              pattern: ctx.pattern.id,
              channel: ctx.channel,
              notes,
            },
            "Paint notes",
            ctx.pattern,
            endOfInits(notes)
          )
        )
        break
      }
      case "erase": {
        if (gesture.ids.size === 0) break
        const notes = [...gesture.ids]
        for (const id of notes) this.selected.delete(id)
        this.commitDrag({
          type: "removeNotes",
          pattern: ctx.pattern.id,
          channel: ctx.channel,
          notes,
        })
        break
      }
      default: {
        const _exhaustive: never = gesture
        return _exhaustive
      }
    }
    this.pointerHover(input)
  }

  /** Drops the gesture in progress and puts everything back. */
  cancel(): void {
    const gesture = this.gesture
    if (gesture.kind === "idle") return
    if (gesture.kind === "marquee") this.selected = new Set(gesture.base)
    this.abandon()
    this.rebuild()
    this.emit("selection")
  }

  /** Stops any note that is sounding and lets go of the canvas. */
  dispose(): void {
    this.abandon()
    this.surface = null
  }

  // Selection

  setSelection(ids: Iterable<number>): void {
    this.applySelection(new Set(ids))
  }

  selectAll(): void {
    this.setSelection(this.scene.notes.map((note) => note.id))
  }

  /** Selects every note on one key. With `add`, keeps what is selected. */
  selectKey(key: number, add: boolean): void {
    const ids = add ? new Set(this.selected) : new Set<number>()
    for (const note of this.scene.notes) {
      if (note.key === key) ids.add(note.id)
    }
    this.applySelection(ids)
  }

  // Edits from the keyboard and menus. Each is one command.

  async deleteSelection(): Promise<void> {
    const ctx = this.ctx
    if (!ctx || this.selected.size === 0) return
    const notes = [...this.selected]
    this.selected = new Set()
    await this.commit({
      type: "removeNotes",
      pattern: ctx.pattern.id,
      channel: ctx.channel,
      notes,
    })
  }

  copy(): boolean {
    const contents = copyNotes(this.selectedNotes())
    if (!contents) return false
    writeClipboard(contents)
    return true
  }

  async cut(): Promise<void> {
    if (this.copy()) await this.deleteSelection()
  }

  /** Pastes the clipboard and selects what was pasted. */
  async paste(target: PasteTarget): Promise<void> {
    const ctx = this.ctx
    const contents = readClipboard()
    if (!ctx || !contents) return
    const start = pasteStart(
      contents,
      target,
      this.host.settings().snap,
      ctx.pattern.signature
    )
    const notes = pasteInits(contents, start)
    await this.commit(
      withExtension(
        {
          type: "addNotes",
          pattern: ctx.pattern.id,
          channel: ctx.channel,
          notes,
        },
        "Paste notes",
        ctx.pattern,
        endOfInits(notes)
      ),
      true
    )
  }

  /** Copies the selection to right after itself and selects the copy. */
  async duplicate(): Promise<void> {
    const ctx = this.ctx
    if (!ctx) return
    const { inits } = duplicateRight(
      this.selectedNotes(),
      this.host.settings().snap
    )
    if (inits.length === 0) return
    await this.commit(
      withExtension(
        {
          type: "addNotes",
          pattern: ctx.pattern.id,
          channel: ctx.channel,
          notes: inits,
        },
        "Duplicate notes",
        ctx.pattern,
        endOfInits(inits)
      ),
      true
    )
  }

  /** Quantizes the selection, or every note when nothing is selected. */
  async quantize(edge: ResizeEdge): Promise<void> {
    const { snap } = this.host.settings()
    const notes = this.targets()
    const updates =
      edge === "start" ? quantizeStarts(notes, snap) : quantizeEnds(notes, snap)
    await this.update(notes, updates, "Quantize notes")
  }

  /** Moves the selection by snap intervals and semitones. */
  async nudge(steps: number, keys: number): Promise<void> {
    const notes = this.selectedNotes()
    const { snap } = this.host.settings()
    // With snap off a nudge is the finest move there is: one tick.
    const delta = moveDelta(
      steps * (snap > 0 ? snap : 1),
      keys,
      0,
      moveLimits(notes)
    )
    await this.update(notes, moveUpdates(notes, delta), "Move notes")
  }

  /** Transposes the selection, or every note when nothing is selected. */
  async transpose(semitones: number): Promise<void> {
    const notes = this.targets()
    const delta = moveDelta(0, semitones, 0, moveLimits(notes))
    await this.update(notes, moveUpdates(notes, delta), "Transpose notes")
  }

  /** The selected notes, or all of them when nothing is selected. */
  targets(): Note[] {
    return this.selected.size > 0
      ? this.selectedNotes()
      : this.scene.notes.filter((note) => note.id >= 0)
  }

  /** Dispatches note updates as one command and follows the result. */
  async update(
    notes: readonly Note[],
    updates: readonly NoteUpdate[],
    label: string
  ): Promise<void> {
    const ctx = this.ctx
    if (!ctx || updates.length === 0) return
    await this.commit(
      withExtension(
        {
          type: "updateNotes",
          pattern: ctx.pattern.id,
          channel: ctx.channel,
          updates: [...updates],
        },
        label,
        ctx.pattern,
        endAfterUpdates(notes, updates)
      )
    )
  }

  // Internals

  private emit(event: EditorEvent): void {
    for (const listener of [...this.listeners]) listener(event)
  }

  private hitAt(input: PointerInput): Hit | null {
    const surface = this.surface
    if (!surface) return null
    return hitTestPoint(surface.viewport, this.scene.items, input.x, input.y)
  }

  private keyAt(y: number): number {
    const surface = this.surface
    if (!surface) return 0
    return clampKey(rowToKey(Math.floor(yToRow(surface.viewport, y))))
  }

  private setHover(tick: number, key: number, intent: Intent | null): void {
    const next: Hover = { tick: Math.max(0, tick), key, intent }
    const previous = this.hover
    this.hover = next
    if (
      previous &&
      Math.round(previous.tick) === Math.round(next.tick) &&
      previous.key === next.key &&
      intentName(previous.intent) === intentName(next.intent)
    ) {
      return
    }
    this.emit("hover")
  }

  private rebuild(): void {
    this.scene = buildScene(
      this.ctx?.notes ?? [],
      this.provisional,
      this.palette,
      this.selected
    )
    this.surface?.setItems(this.scene.items)
    this.emit("scene")
  }

  /** Changes the selection in place: flags only, no rebuild. */
  private applySelection(ids: Set<number>): void {
    this.selected = ids
    const batch = this.scene.items.batch
    const indices: number[] = []
    for (const id of ids) {
      const index = batch.indexOfId(id)
      if (index >= 0) indices.push(index)
    }
    batch.setSelection(indices)
    this.surface?.invalidate("base")
    this.emit("selection")
  }

  private toggleSelected(id: number): void {
    const ids = new Set(this.selected)
    if (!ids.delete(id)) ids.add(id)
    this.applySelection(ids)
  }

  private sound(key: number | null, velocity?: number): void {
    const ctx = this.ctx
    if (key === this.sounding) return
    if (this.sounding !== null && ctx) {
      this.host.noteOff(ctx.channel, this.sounding)
    }
    this.sounding = null
    if (key === null || !ctx || key < 0 || key > MAX_KEY) return
    this.sounding = key
    this.host.noteOn(
      ctx.channel,
      key,
      velocity ?? this.host.settings().lastVelocity
    )
  }

  private clearPreview(): void {
    this.surface?.setDragOffset(0, 0)
    this.surface?.setDragResize(0, 0)
    if (this.preview !== null) {
      this.preview = null
      this.emit("drag")
    }
  }

  /** Ends whatever is in progress without committing it. No rebuild. */
  private abandon(): void {
    this.sound(null)
    this.gesture = IDLE
    if (this.provisional.length > 0) {
      for (const note of this.provisional) this.selected.delete(note.id)
      this.provisional = []
    }
    this.surface?.setMarquee(null)
    this.clearPreview()
  }

  /**
   * Dispatches a command and follows the project to its result. With
   * `selectCreated` the notes it made become the selection.
   */
  private async commit(command: Command, selectCreated = false): Promise<void> {
    const result = await this.host.dispatch(command)
    if (result && selectCreated) this.selected = new Set(result.created)
    // Forced, because a command that failed leaves the lane as it was and
    // the preview still has to go.
    this.follow(this.host.context(), true)
  }

  /**
   * Commits the end of a drag. The preview stays up until the project
   * answers, so the notes never jump back for a frame.
   */
  private commitDrag(command: Command, selectCreated = false): void {
    void this.commit(command, selectCreated)
  }

  private beginMove(
    hit: Hit,
    input: PointerInput,
    press: Press,
    tick: number,
    row: number
  ): void {
    const grabbed = this.scene.notes[hit.index]
    const wasSelected = this.selected.has(grabbed.id)
    if (!wasSelected) {
      this.setSelection(
        input.shift ? [...this.selected, grabbed.id] : [grabbed.id]
      )
    }
    const notes = this.selectedNotes()
    this.gesture = {
      kind: "move",
      press,
      tick,
      row,
      grabbed,
      notes,
      limits: moveLimits(notes),
      delta: { ticks: 0, keys: 0 },
      wasSelected,
      shift: input.shift,
      provisional: false,
      moved: false,
    }
    this.host.remember(grabbed.length, grabbed.velocity)
    this.sound(grabbed.key, grabbed.velocity)
  }

  private finishMove(
    gesture: Extract<Gesture, { kind: "move" }>,
    input: PointerInput,
    ctx: EditorContext
  ): void {
    const { delta, notes } = gesture
    const target = { pattern: ctx.pattern.id, channel: ctx.channel }
    if (gesture.provisional) {
      const inits = movedCopies(notes, delta)
      this.selected = new Set()
      this.commitDrag(
        withExtension(
          { type: "addNotes", ...target, notes: inits },
          "Add note",
          ctx.pattern,
          endOfInits(inits)
        )
      )
      return
    }
    if (delta.ticks === 0 && delta.keys === 0) {
      this.clearPreview()
      if (gesture.moved) return
      // A click, not a drag: it only changes what is selected.
      if (gesture.shift && gesture.wasSelected) {
        this.toggleSelected(gesture.grabbed.id)
      } else if (!gesture.shift && this.selected.size > 1) {
        this.setSelection([gesture.grabbed.id])
      }
      return
    }
    if (input.ctrl) {
      const inits = movedCopies(notes, delta)
      this.commitDrag(
        withExtension(
          { type: "addNotes", ...target, notes: inits },
          "Duplicate notes",
          ctx.pattern,
          endOfInits(inits)
        ),
        true
      )
      return
    }
    const updates = moveUpdates(notes, delta)
    this.commitDrag(
      withExtension(
        { type: "updateNotes", ...target, updates },
        "Move notes",
        ctx.pattern,
        endAfterUpdates(notes, updates)
      )
    )
  }

  private beginResize(
    hit: Hit,
    edge: ResizeEdge,
    input: PointerInput,
    press: Press,
    tick: number
  ): void {
    const grabbed = this.scene.notes[hit.index]
    if (!this.selected.has(grabbed.id)) {
      this.setSelection(
        input.shift ? [...this.selected, grabbed.id] : [grabbed.id]
      )
    }
    const notes = this.selectedNotes()
    this.gesture = {
      kind: "resize",
      press,
      tick,
      edge,
      grabbed,
      notes,
      delta: { start: 0, end: 0, minLength: 1 },
      moved: false,
    }
  }

  private beginDraw(
    input: PointerInput,
    press: Press,
    tick: number,
    row: number,
    settings: EditorSettings
  ): void {
    const key = rowToKey(Math.floor(row))
    if (key < 0 || key > MAX_KEY) return
    const snap = noSnap(input) ? 0 : settings.snap
    const note: Note = {
      id: PROVISIONAL_ID,
      start: Math.max(0, snapFloor(tick, snap)),
      length: Math.max(1, settings.lastLength),
      key,
      velocity: settings.lastVelocity,
      pan: 0,
    }
    this.provisional = [note]
    this.selected = new Set([note.id])
    this.rebuild()
    this.emit("selection")
    this.gesture = {
      kind: "move",
      press,
      tick,
      row,
      grabbed: note,
      notes: [note],
      limits: moveLimits([note]),
      delta: { ticks: 0, keys: 0 },
      wasSelected: false,
      shift: false,
      provisional: true,
      moved: false,
    }
    this.sound(key, note.velocity)
  }

  private beginPaint(
    input: PointerInput,
    tick: number,
    row: number,
    settings: EditorSettings
  ): void {
    const snap = noSnap(input) ? 0 : settings.snap
    const gesture: Extract<Gesture, { kind: "paint" }> = {
      kind: "paint",
      origin: Math.max(0, snapFloor(tick, snap)),
      spacing: paintSpacing(snap, settings.lastLength),
      length: Math.max(1, settings.lastLength),
      velocity: settings.lastVelocity,
      cells: new Map(),
      lastCell: 0,
    }
    this.gesture = gesture
    if (this.selected.size > 0) this.setSelection([])
    this.paintCells(gesture, [0], row)
  }

  private paintCells(
    gesture: Extract<Gesture, { kind: "paint" }>,
    cells: readonly number[],
    row: number
  ): void {
    const key = rowToKey(Math.floor(row))
    if (key < 0 || key > MAX_KEY) return
    const keyRow = keyToRow(key)
    let added = false
    for (const cell of cells) {
      if (gesture.cells.has(cell)) continue
      const start = gesture.origin + cell * gesture.spacing
      if (start < 0) continue
      // A cell that already holds a note on this key is left alone.
      const taken = queryRect(
        this.scene.items,
        start,
        start + 1,
        keyRow,
        keyRow + 1
      )
      if (taken.length > 0) continue
      gesture.cells.set(cell, {
        id: PROVISIONAL_ID - gesture.cells.size,
        start,
        length: gesture.length,
        key,
        velocity: gesture.velocity,
        pan: 0,
      })
      added = true
    }
    if (!added) return
    this.provisional = [...gesture.cells.values()]
    this.rebuild()
    this.sound(null)
    this.sound(key, gesture.velocity)
  }

  private markErased(indices: readonly number[]): void {
    const gesture = this.gesture
    if (gesture.kind !== "erase") return
    const batch = this.scene.items.batch
    let changed = false
    for (const index of indices) {
      const id = batch.ids[index]
      if (gesture.ids.has(id)) continue
      gesture.ids.add(id)
      const c = index * 4
      batch.setColor(
        index,
        withAlpha(
          {
            r: batch.colors[c],
            g: batch.colors[c + 1],
            b: batch.colors[c + 2],
            a: batch.colors[c + 3],
          },
          ERASED_ALPHA
        )
      )
      changed = true
    }
    if (!changed) return
    this.surface?.invalidate("base")
    this.emit("drag")
  }

  private eraseAlong(
    tick0: number,
    row0: number,
    tick1: number,
    row1: number
  ): void {
    for (const span of rowsAlongSegment(tick0, row0, tick1, row1)) {
      // A pointer that has not moved covers no time, so give it a sliver.
      const end = Math.max(span.tick1, span.tick0 + 0.001)
      this.markErased(
        queryRect(this.scene.items, span.tick0, end, span.row, span.row + 1)
      )
    }
  }
}

function pastDeadZone(press: Press, input: PointerInput): boolean {
  return (
    Math.abs(input.x - press.x) > DEAD_ZONE_PX ||
    Math.abs(input.y - press.y) > DEAD_ZONE_PX
  )
}

function intentName(intent: Intent | null): string {
  if (!intent) return ""
  return intent.kind === "resize" ? `resize-${intent.edge}` : intent.kind
}
