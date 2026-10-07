import { useEffect } from "react"

import { activeScopeOf, useUiStore, type ScopeId } from "@/lib/store/ui"

import type { EditCommand } from "./registry"

/*
 * Which panel the keyboard belongs to.
 *
 * A panel marks its root element with `useShortcutScope`. A key pressed
 * while the focus is inside that element goes to the panel's own shortcuts
 * first, then to those of any scope around it, then to the global ones.
 *
 * A part of a panel can be a scope of its own inside the panel's: the
 * channel settings beside the rack, the effects beside the mixer. It may
 * keep some of the Edit commands from the scopes around it. Delete with the
 * focus on a channel's settings then does not go on to delete the channel.
 *
 * The focus is often on nothing a panel owns: on the page itself after a
 * click on a canvas, on a transport button, in a menu. Then the keys go to
 * the scope that was clicked or focused last, and with none yet to the
 * center tab in view. A dialog is the exception: it takes the keyboard for
 * itself, so only global shortcuts work while the focus is in one.
 */

const SCOPE_ATTRIBUTE = "data-shortcut-scope"
const KEEPS_ATTRIBUTE = "data-shortcut-keeps"
const MODAL_SELECTOR = "[role='dialog'], [role='alertdialog']"

/** One scope on the way from the focused element outward. */
export type ScopeLink = {
  scope: ScopeId
  /**
   * Edit commands this scope keeps from the scopes around it. A key that
   * would run one of those out there does nothing instead.
   */
  keeps: readonly EditCommand[]
}

function linksAround(target: EventTarget | null): ScopeLink[] {
  const links: ScopeLink[] = []
  let element =
    target instanceof Element ? target.closest(`[${SCOPE_ATTRIBUTE}]`) : null
  while (element) {
    const scope = element.getAttribute(SCOPE_ATTRIBUTE) as ScopeId
    if (!links.some((link) => link.scope === scope)) {
      const keeps = element.getAttribute(KEEPS_ATTRIBUTE)
      links.push({
        scope,
        keeps: keeps ? (keeps.split(" ") as EditCommand[]) : [],
      })
    }
    element = element.parentElement?.closest(`[${SCOPE_ATTRIBUTE}]`) ?? null
  }
  return links
}

/** The scopes from `active` outward, as its element on the page has them. */
export function linksOfScope(active: ScopeId): ScopeLink[] {
  const root = document.querySelector(`[${SCOPE_ATTRIBUTE}="${active}"]`)
  return root ? linksAround(root) : [{ scope: active, keeps: [] }]
}

/**
 * The scopes a key pressed on `target` goes through, innermost first, each
 * with what it keeps from the ones after it.
 */
export function scopeLinks(target: EventTarget | null): ScopeLink[] {
  const around = linksAround(target)
  if (around.length > 0) return around
  if (target instanceof Element && target.closest(MODAL_SELECTOR)) return []
  return linksOfScope(activeScopeOf(useUiStore.getState()))
}

/** The scopes around an element, innermost first. */
export function scopesAround(target: EventTarget | null): ScopeId[] {
  return linksAround(target).map((link) => link.scope)
}

/** The scopes a key pressed on `target` goes through, innermost first. */
export function scopeChain(target: EventTarget | null): ScopeId[] {
  return scopeLinks(target).map((link) => link.scope)
}

/** Remembers the scope a click or a focus change landed in. */
function trackScope(event: Event) {
  const [scope] = scopesAround(event.target)
  if (scope === undefined) return
  const ui = useUiStore.getState()
  if (ui.activeScope !== scope) ui.setActiveScope(scope)
}

/** Follows clicks and the focus to know the active scope. Returns a function that stops. */
export function installScopeTracking(): () => void {
  document.addEventListener("pointerdown", trackScope, true)
  document.addEventListener("focusin", trackScope, true)
  return () => {
    document.removeEventListener("pointerdown", trackScope, true)
    document.removeEventListener("focusin", trackScope, true)
  }
}

type ScopeOptions = {
  /**
   * Edit commands to keep from the scopes around this one. With
   * `["delete", "duplicate"]`, Delete and Ctrl+D pressed in here never
   * reach the panel's "Delete channel" or "Duplicate channel".
   */
  keeps?: readonly EditCommand[]
}

/**
 * What an inspector keeps from its panel: the commands that would delete or
 * copy the very thing whose settings are being edited.
 */
export const INSPECTOR_KEEPS: readonly EditCommand[] = [
  "delete",
  "duplicate",
  "cut",
]

/**
 * Makes an element the root of a shortcut scope. Spread the result on the
 * outermost element of the panel, or of the part of it:
 *
 *     const scope = useShortcutScope("mixer")
 *     return <div {...scope}>…</div>
 *
 * Actions registered with `scope: "mixer"` then get their keys while the
 * focus is inside, or after the panel was the last one clicked.
 */
export function useShortcutScope(scope: ScopeId, options: ScopeOptions = {}) {
  useEffect(
    () => () => {
      // A scope that is gone cannot keep the keyboard.
      const ui = useUiStore.getState()
      if (ui.activeScope === scope) ui.setActiveScope(null)
    },
    [scope]
  )
  const keeps = options.keeps?.join(" ")
  return {
    [SCOPE_ATTRIBUTE]: scope,
    [KEEPS_ATTRIBUTE]: keeps || undefined,
  } as {
    "data-shortcut-scope": ScopeId
    "data-shortcut-keeps": string | undefined
  }
}
