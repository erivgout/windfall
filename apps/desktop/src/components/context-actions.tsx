import type { ReactElement } from "react"

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuGroup,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"
import {
  isEnabled,
  runAction,
  shortcutLabel,
  useActions,
  useAppState,
} from "@/lib/actions"
import { reportError } from "@/lib/errors"

/** An entry that is not in the registry because it acts on the thing clicked. */
export type InlineAction = {
  title: string
  run(): void | Promise<void>
  disabled?: boolean
  destructive?: boolean
  /** Shown as a hint only. Bind real keys through the registry. */
  shortcut?: string
}

export type ContextItem =
  /** A registry action id. */
  string | InlineAction | { separator: true } | { label: string }

export const contextSeparator = { separator: true } as const

type ContextActionsProps = {
  /** What the menu offers, top to bottom. */
  items: ContextItem[]
  /** The element that opens the menu on right-click. Must take a ref. */
  children: ReactElement
}

/**
 * Gives an element a right-click menu. Entries are registry action ids or
 * inline actions for the thing that was clicked:
 *
 *     <ContextActions
 *       items={[
 *         { title: "Rename", run: rename },
 *         contextSeparator,
 *         "channel.add",
 *       ]}
 *     >
 *       <div>…</div>
 *     </ContextActions>
 */
export function ContextActions({ items, children }: ContextActionsProps) {
  return (
    <ContextMenu>
      <ContextMenuTrigger render={children} />
      <ContextMenuContent className="min-w-44">
        <ContextItems items={items} />
      </ContextMenuContent>
    </ContextMenu>
  )
}

// Rendered only while the menu is open, so reading the whole app state here
// costs nothing the rest of the time.
function ContextItems({ items }: { items: ContextItem[] }) {
  const actions = useActions()
  const state = useAppState()

  return items.map((item, index) => {
    if (typeof item === "string") {
      const action = actions.find((candidate) => candidate.id === item)
      if (!action) return null
      const shortcut = shortcutLabel(item)
      return (
        <ContextMenuItem
          key={item}
          disabled={!isEnabled(action, state)}
          onClick={() => void runAction(item)}
        >
          {action.title}
          {shortcut && <ContextMenuShortcut>{shortcut}</ContextMenuShortcut>}
        </ContextMenuItem>
      )
    }
    if ("separator" in item) return <ContextMenuSeparator key={index} />
    if ("label" in item) {
      return (
        <ContextMenuGroup key={index}>
          <ContextMenuLabel>{item.label}</ContextMenuLabel>
        </ContextMenuGroup>
      )
    }
    return (
      <ContextMenuItem
        key={index}
        disabled={item.disabled}
        variant={item.destructive ? "destructive" : "default"}
        onClick={() => {
          new Promise((resolve) => resolve(item.run())).catch(
            (error: unknown) => reportError(error, item.title)
          )
        }}
      >
        {item.title}
        {item.shortcut && (
          <ContextMenuShortcut>{item.shortcut}</ContextMenuShortcut>
        )}
      </ContextMenuItem>
    )
  })
}
