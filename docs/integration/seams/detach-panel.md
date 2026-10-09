# Detached panels (wf-detached-windows)

Each detached panel is a second window on the same backend. The Browser,
Mixer, Channel rack, Playlist, and Piano roll headers offer Detach; the
panel's own window offers Dock. Closing that window also docks the panel.

The choice is session state and is not saved. Replacing the project keeps
the panel windows open, following the new project through the shared backend.
Detaching hides the panel in the main workspace without changing its saved
visibility or preferred center tab. Docking restores it under those preferences.
When the preferred center tab is detached, another available editor shows;
when all editors are detached, the center explains that they are in their own windows.

`detachPanel(id)` opens or focuses the Tauri window labeled `panel-<id>`.
The main workspace marks the panel detached only after that call succeeds.
`dockPanel(id)` closes the window. Its destruction emits `panel-docked` to
the main window, where `onPanelDocked` removes the session marker. Repeated
detach requests and duplicate dock events are harmless. Unknown ids are rejected.
