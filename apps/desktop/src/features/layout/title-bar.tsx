import { Moon02Icon, Search01Icon, Sun03Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import { ActionButton } from "@/components/action-button"
import { Kbd } from "@/components/ui/kbd"
import { runAction, useShortcutLabel } from "@/lib/actions"
import { useHint } from "@/lib/store/hint"
import { useDirty, useProjectName } from "@/lib/store/selectors"
import { resolveTheme, useUiStore } from "@/lib/store/ui"

import { AppMenuBar } from "./menu-bar"

function Mark() {
  return (
    <span
      aria-hidden
      className="flex size-[1.125rem] shrink-0 items-center justify-center rounded-[5px] bg-brand text-[0.6875rem] font-semibold text-brand-foreground"
    >
      W
    </span>
  )
}

function ProjectTitle() {
  const name = useProjectName()
  const dirty = useDirty()
  return (
    <div className="pointer-events-none absolute inset-x-0 flex justify-center">
      <span className="flex max-w-[30%] items-center gap-1.5 text-foreground/70">
        <span className="truncate">{name || "Untitled"}</span>
        {dirty && (
          <span
            role="img"
            aria-label="Unsaved changes"
            className="size-1.5 shrink-0 rounded-full bg-brand"
          />
        )}
      </span>
    </div>
  )
}

function PaletteButton() {
  const shortcut = useShortcutLabel("view.commandPalette")
  const hint = useHint("Find any action by name and see its shortcut")
  return (
    <button
      type="button"
      onClick={() => void runAction("view.commandPalette")}
      className="flex h-6 w-52 items-center gap-1.5 rounded-md border border-input/70 bg-background/60 pr-1 pl-2 text-muted-foreground outline-none hover:border-input hover:text-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
      {...hint}
    >
      <HugeiconsIcon icon={Search01Icon} strokeWidth={2} className="size-3" />
      <span className="flex-1 text-left">Search actions</span>
      {shortcut && <Kbd className="h-4 min-w-4">{shortcut}</Kbd>}
    </button>
  )
}

function ThemeButton() {
  const dark = useUiStore((state) => resolveTheme(state.theme) === "dark")
  return (
    <ActionButton action="view.toggleTheme" variant="ghost" size="icon-sm">
      <HugeiconsIcon icon={dark ? Sun03Icon : Moon02Icon} strokeWidth={2} />
    </ActionButton>
  )
}

/** The top strip: menus on the left, the project name, and search on the right. */
export function TitleBar() {
  return (
    <header className="relative flex h-8 shrink-0 items-center gap-2 border-b bg-chassis pr-1.5 pl-2">
      <Mark />
      <AppMenuBar />
      <ProjectTitle />
      <div className="relative ml-auto flex items-center gap-1">
        <PaletteButton />
        <ThemeButton />
      </div>
    </header>
  )
}
