import { useState } from "react"

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "@/components/ui/command"
import {
  disabledReason,
  isEnabled,
  runAction,
  shortcutLabel,
  useActions,
  useAppState,
  type Action,
} from "@/lib/actions"
import { useUiStore } from "@/lib/store/ui"

import { rank } from "./score"

function bySection(actions: Action[]): [string, Action[]][] {
  const sections = new Map<string, Action[]>()
  for (const action of actions) {
    sections.set(action.section, [
      ...(sections.get(action.section) ?? []),
      action,
    ])
  }
  return [...sections]
}

/** The actions a search finds, best first: what the palette lists for it. */
export function searchActions(actions: readonly Action[], search: string) {
  return rank(actions, search, (action) => ({
    title: action.title,
    keywords: `${action.section} ${action.keywords ?? ""}`,
  }))
}

type ListProps = {
  search: string
  onRun(id: string): void
}

// Mounted only while the palette is open, so it can follow the app state.
function PaletteList({ search, onRun }: ListProps) {
  const actions = useActions()
  const state = useAppState()
  const searching = search.trim() !== ""

  const row = (action: Action, where?: string) => {
    const shortcut = shortcutLabel(action.id)
    const reason = disabledReason(action, state)
    return (
      <CommandItem
        key={action.id}
        value={action.id}
        disabled={!isEnabled(action, state)}
        onSelect={() => onRun(action.id)}
      >
        {action.title.replace(/…$/, "")}
        {where && (
          <span className="text-[0.625rem] text-muted-foreground">{where}</span>
        )}
        {reason ? (
          <CommandShortcut className="tracking-normal">
            {reason}
          </CommandShortcut>
        ) : (
          shortcut && <CommandShortcut>{shortcut}</CommandShortcut>
        )}
      </CommandItem>
    )
  }

  return (
    <CommandList className="max-h-[min(24rem,60vh)]">
      <CommandEmpty>No action matches that.</CommandEmpty>
      {searching ? (
        // One list, best match first. Under their headings the best match
        // could sit below a whole section of worse ones.
        <CommandGroup>
          {searchActions(actions, search).map((action) =>
            row(action, action.section)
          )}
        </CommandGroup>
      ) : (
        bySection(actions).map(([section, items]) => (
          <CommandGroup key={section} heading={section}>
            {items.map((action) => row(action))}
          </CommandGroup>
        ))
      )}
    </CommandList>
  )
}

/**
 * Every action in the registry in one searchable list, each with its
 * shortcut. Choosing one runs it.
 */
export function CommandPalette() {
  const open = useUiStore((state) => state.dialog === "palette")
  const closeDialog = useUiStore((state) => state.closeDialog)
  const [search, setSearch] = useState("")
  // A palette opened again starts with an empty search.
  const [wasOpen, setWasOpen] = useState(open)
  if (open !== wasOpen) {
    setWasOpen(open)
    if (open) setSearch("")
  }

  function run(id: string) {
    closeDialog()
    void runAction(id)
  }

  return (
    <CommandDialog
      open={open}
      onOpenChange={(next) => {
        if (!next) closeDialog()
      }}
      title="Command palette"
      description="Search for an action to run."
      className="sm:max-w-lg"
    >
      {/* cmdk's vim keys would swallow Ctrl+K, which closes the palette.
          The list is filtered and put in order here, not by cmdk: it sorts
          sections as wholes, and loses track of some of them. */}
      <Command vimBindings={false} shouldFilter={false}>
        <CommandInput
          placeholder="Search actions…"
          value={search}
          onValueChange={setSearch}
        />
        <PaletteList search={search} onRun={run} />
      </Command>
    </CommandDialog>
  )
}
