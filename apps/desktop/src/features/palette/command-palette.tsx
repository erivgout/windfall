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
  isEnabled,
  runAction,
  shortcutLabel,
  useActions,
  useAppState,
  type Action,
} from "@/lib/actions"
import { useUiStore } from "@/lib/store/ui"

import { scoreAction } from "./score"

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

// Mounted only while the palette is open, so it can follow the app state.
function PaletteList({ onRun }: { onRun(id: string): void }) {
  const actions = useActions()
  const state = useAppState()

  return (
    <CommandList className="max-h-[min(24rem,60vh)]">
      <CommandEmpty>No action matches that.</CommandEmpty>
      {bySection(actions).map(([section, items]) => (
        <CommandGroup key={section} heading={section}>
          {items.map((action) => {
            const shortcut = shortcutLabel(action.id)
            return (
              <CommandItem
                key={action.id}
                value={action.id}
                // The filter below reads the title and the search words here.
                keywords={[action.title, `${section} ${action.keywords ?? ""}`]}
                disabled={!isEnabled(action, state)}
                onSelect={() => onRun(action.id)}
              >
                {action.title.replace(/…$/, "")}
                {shortcut && <CommandShortcut>{shortcut}</CommandShortcut>}
              </CommandItem>
            )
          })}
        </CommandGroup>
      ))}
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
      {/* cmdk's vim keys would swallow Ctrl+K, which closes the palette. */}
      <Command
        vimBindings={false}
        filter={(value, search, keywords = []) =>
          scoreAction(keywords[0] ?? value, keywords[1] ?? "", search)
        }
      >
        <CommandInput placeholder="Search actions…" />
        <PaletteList onRun={run} />
      </Command>
    </CommandDialog>
  )
}
