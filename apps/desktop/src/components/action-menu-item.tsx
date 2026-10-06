import { Tick02Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"

import {
  DropdownMenuItem,
  DropdownMenuShortcut,
} from "@/components/ui/dropdown-menu"
import {
  isChecked,
  isEnabled,
  runAction,
  shortcutLabel,
  type Action,
  type AppState,
} from "@/lib/actions"

type ActionMenuItemProps = {
  action: Action
  state: AppState
  /** Leaves room for a tick, to line up with on/off items in the same menu. */
  inset?: boolean
}

/**
 * One registry action as an item of a dropdown menu or the menu bar. Title,
 * shortcut, on/off tick and disabled state all come from the registry.
 */
export function ActionMenuItem({ action, state, inset }: ActionMenuItemProps) {
  const shortcut = shortcutLabel(action.id)
  const checkable = action.checked !== undefined
  const checked = isChecked(action, state)

  return (
    <DropdownMenuItem
      role={checkable ? "menuitemcheckbox" : undefined}
      aria-checked={checkable ? checked : undefined}
      disabled={!isEnabled(action, state)}
      onClick={() => void runAction(action.id)}
    >
      {(checkable || inset) && (
        <span className="flex size-3.5 items-center justify-center" aria-hidden>
          {checked && <HugeiconsIcon icon={Tick02Icon} strokeWidth={2} />}
        </span>
      )}
      <span className="whitespace-nowrap">{action.title}</span>
      {shortcut && (
        <DropdownMenuShortcut className="pl-6">{shortcut}</DropdownMenuShortcut>
      )}
    </DropdownMenuItem>
  )
}
