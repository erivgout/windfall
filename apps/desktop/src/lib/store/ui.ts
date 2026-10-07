import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ChannelId, TrackId } from "@/bindings"

export type Theme = "dark" | "light" | "system"
export type KeymapPreset = "windfall" | "fl"
/** Panels docked around the center that can be shown and hidden. */
export type SidePanel = "browser" | "mixer"
/** Panels that share the center area as tabs. */
export type CenterTab = "channelRack" | "playlist" | "pianoRoll"
/** Any panel. A panel is also the scope its keyboard shortcuts are live in. */
export type PanelId = SidePanel | CenterTab
/**
 * A part of a panel with keys of its own, inside the panel's scope: the
 * channel settings beside the rack, the effects beside the mixer, one
 * effect (its slot on a strip, or the header of its panel), and the
 * settings of the selected audio clips above the timeline.
 */
export type InnerScope =
  "rackInspector" | "effectInspector" | "effect" | "clipInspector"
/** Anywhere keyboard shortcuts can be scoped to. */
export type ScopeId = PanelId | InnerScope
export type AppDialog =
  | "tempoTap"
  | "palette"
  | "settings"
  | "export"
  | "midiImport"
  | "midiExport"
  | "flpImport"
  | "flpRetained"
/**
 * Something that takes the place of the center tab for a while. The tab
 * stays chosen underneath and comes back when the overlay goes.
 */
export type CenterOverlay = "effects"
/** Panel sizes of one resizable group, as percentages by panel id. */
export type PanelSizes = Record<string, number>

type UiState = {
  theme: Theme
  keymap: KeymapPreset
  panels: Record<SidePanel, boolean>
  centerTab: CenterTab
  /** What is showing over the center tab, if anything. */
  centerOverlay: CenterOverlay | null
  /** Saved sizes, keyed by group and by which panels were showing. */
  layouts: Record<string, PanelSizes>
  /** Bumped by "Reset layout" so the panel groups start over. */
  layoutGeneration: number
  selectedChannel: ChannelId | null
  selectedTrack: TrackId | null
  /**
   * The panel, or part of one, last clicked or focused, which is where keys
   * go while the focus is on nothing in particular. Null means the center
   * tab in view.
   */
  activeScope: ScopeId | null
  dialog: AppDialog | null
  historyOpen: boolean

  setTheme(theme: Theme): void
  toggleTheme(): void
  setKeymap(keymap: KeymapPreset): void
  togglePanel(panel: SidePanel): void
  setPanelVisible(panel: SidePanel, visible: boolean): void
  showCenterTab(tab: CenterTab): void
  setCenterOverlay(overlay: CenterOverlay | null): void
  saveLayout(key: string, sizes: PanelSizes): void
  /** Forgets the sizes saved for one group, so its divider starts over. */
  resetPanelSizes(group: string): void
  resetLayout(): void
  selectChannel(id: ChannelId | null): void
  selectTrack(id: TrackId | null): void
  setActiveScope(scope: ScopeId | null): void
  setHistoryOpen(open: boolean): void
  openDialog(dialog: AppDialog): void
  closeDialog(): void
}

const DEFAULT_PANELS: Record<SidePanel, boolean> = {
  browser: true,
  mixer: true,
}

export function resolveTheme(theme: Theme): "dark" | "light" {
  if (theme !== "system") return theme
  const prefersDark =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  return prefersDark ? "dark" : "light"
}

/** The scope whose shortcuts are live right now. */
export function activeScopeOf(state: {
  activeScope: ScopeId | null
  centerTab: CenterTab
}): ScopeId {
  return state.activeScope ?? state.centerTab
}

/**
 * What the user is looking at and how they like the app set up. Preferences
 * (theme, keymap, panels and their sizes) survive a restart; the selection
 * and open dialogs do not.
 */
export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      theme: "dark",
      keymap: "windfall",
      panels: DEFAULT_PANELS,
      centerTab: "channelRack",
      centerOverlay: null,
      layouts: {},
      layoutGeneration: 0,
      selectedChannel: null,
      selectedTrack: null,
      activeScope: null,
      dialog: null,
      historyOpen: false,

      setTheme: (theme) => set({ theme }),
      toggleTheme: () =>
        set((state) => ({
          theme: resolveTheme(state.theme) === "dark" ? "light" : "dark",
        })),
      setKeymap: (keymap) => set({ keymap }),
      togglePanel: (panel) =>
        set((state) => ({
          panels: { ...state.panels, [panel]: !state.panels[panel] },
        })),
      setPanelVisible: (panel, visible) =>
        set((state) => ({ panels: { ...state.panels, [panel]: visible } })),
      // Bringing an editor forward also hands it the keyboard, and puts
      // away whatever was lying over the editors.
      showCenterTab: (centerTab) =>
        set({ centerTab, centerOverlay: null, activeScope: null }),
      setCenterOverlay: (centerOverlay) => set({ centerOverlay }),
      saveLayout: (key, sizes) =>
        set((state) => ({ layouts: { ...state.layouts, [key]: sizes } })),
      resetPanelSizes: (group) =>
        set((state) => ({
          layouts: Object.fromEntries(
            Object.entries(state.layouts).filter(
              ([key]) => !key.startsWith(`${group}:`)
            )
          ),
          // The groups read their sizes when they are made.
          layoutGeneration: state.layoutGeneration + 1,
        })),
      resetLayout: () =>
        set((state) => ({
          panels: DEFAULT_PANELS,
          centerTab: "channelRack",
          centerOverlay: null,
          activeScope: null,
          layouts: {},
          layoutGeneration: state.layoutGeneration + 1,
        })),
      selectChannel: (selectedChannel) => set({ selectedChannel }),
      selectTrack: (selectedTrack) => set({ selectedTrack }),
      setActiveScope: (activeScope) => set({ activeScope }),
      setHistoryOpen: (historyOpen) => set({ historyOpen }),
      openDialog: (dialog) => set({ dialog }),
      closeDialog: () => set({ dialog: null }),
    }),
    {
      name: "windfall.ui",
      version: 1,
      partialize: (state) => ({
        theme: state.theme,
        keymap: state.keymap,
        panels: state.panels,
        centerTab: state.centerTab,
        layouts: state.layouts,
      }),
    }
  )
)
