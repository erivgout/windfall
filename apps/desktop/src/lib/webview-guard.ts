import { eventChord, isTypingTarget } from "@/lib/actions/keymap"

/*
 * The app runs in a webview, which is a browser and has a browser's habits:
 * a right-click menu with "Reload" and "Inspect", F5 to reload, Ctrl+P to
 * print, Ctrl+wheel to zoom the page. None of them belong in the app. This
 * turns them off in one place.
 *
 * It listens on the window after everything else, so the app's own menus
 * and shortcuts have already had the event. A browser key that is also an
 * app shortcut runs the app's action; one that is not is swallowed.
 */

/** Keys the webview acts on by itself, in the spelling of `eventChord`. */
const BROWSER_KEYS = new Set([
  // Reload.
  "F5",
  "Ctrl+F5",
  "Shift+F5",
  "Ctrl+R",
  "Ctrl+Shift+R",
  // Find, caret browsing, print, downloads, view source.
  "F3",
  "Shift+F3",
  "Ctrl+G",
  "Ctrl+Shift+G",
  "F7",
  "Ctrl+P",
  "Ctrl+Shift+P",
  "Ctrl+J",
  "Ctrl+U",
  // Page zoom.
  "Ctrl+=",
  "Ctrl++",
  "Ctrl+Shift+=",
  "Ctrl+Shift++",
  "Ctrl+-",
  "Ctrl+0",
  "Ctrl+NumpadAdd",
  "Ctrl+NumpadSubtract",
  "Ctrl+Numpad0",
  // Back and forward.
  "Alt+ArrowLeft",
  "Alt+ArrowRight",
  "BrowserBack",
  "BrowserForward",
])

function onContextMenu(event: MouseEvent) {
  // A text field keeps the menu, for its cut, copy and paste.
  if (!isTypingTarget(event.target)) event.preventDefault()
}

function onKeyDown(event: KeyboardEvent) {
  if (event.defaultPrevented) return
  // Cmd does on macOS what Ctrl does elsewhere.
  const chord = eventChord({
    key: event.key,
    code: event.code,
    ctrlKey: event.ctrlKey || event.metaKey,
    altKey: event.altKey,
    shiftKey: event.shiftKey,
    metaKey: false,
  })
  if (chord === null) return
  // Backspace used to go back a page, and still does in some webviews.
  const goesBack = chord === "Backspace" && !isTypingTarget(event.target)
  if (goesBack || BROWSER_KEYS.has(chord)) event.preventDefault()
}

function onWheel(event: WheelEvent) {
  // An editor that zooms with Ctrl+wheel has taken the event already.
  if (event.ctrlKey) event.preventDefault()
}

/**
 * Keeps the webview's own menu, keys and zoom out of the app. Returns a
 * function that lets them back in.
 */
export function installWebviewGuard(): () => void {
  window.addEventListener("contextmenu", onContextMenu)
  window.addEventListener("keydown", onKeyDown)
  window.addEventListener("wheel", onWheel, { passive: false })
  return () => {
    window.removeEventListener("contextmenu", onContextMenu)
    window.removeEventListener("keydown", onKeyDown)
    window.removeEventListener("wheel", onWheel)
  }
}
