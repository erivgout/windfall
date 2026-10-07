/** What text is typed into. A right-click there is the field's own. */
export const TEXT_FIELD_SELECTOR =
  "input, textarea, [contenteditable=''], [contenteditable='true']"

export function isTextField(target: EventTarget | null): boolean {
  return (
    target instanceof Element && target.closest(TEXT_FIELD_SELECTOR) !== null
  )
}

/*
 * A text field keeps the menu every text field has, for its cut, copy and
 * paste. The app's own right-click menus cover whole panels, and the menu
 * component they are made of switches the native menu off for everything
 * inside one. So a right-click on a text field is taken out of the way
 * before any of them sees it: caught at the window on its way down, and
 * sent no further.
 */
function onContextMenu(event: MouseEvent) {
  if (isTextField(event.target)) event.stopPropagation()
}

/** Leaves right-clicks on text fields to the fields. Returns a function that stops. */
export function installTextFieldMenu(): () => void {
  window.addEventListener("contextmenu", onContextMenu, true)
  return () => window.removeEventListener("contextmenu", onContextMenu, true)
}
