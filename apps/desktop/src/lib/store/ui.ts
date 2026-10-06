import { create } from "zustand"
import { persist } from "zustand/middleware"

import type { ChannelId, TrackId } from "@/bindings"

export type Theme = "dark" | "light" | "system"
export type KeymapPreset = "windfall" | "fl"
/** Panels docked around the center that can be shown and hidden. */
export type SidePanel = "browser" | "mixer"
/** Panels that share the center area as tabs. */
export type CenterTab = "channelRack" | "playlist" | "pianoRoll"
export type AppDialog = "palette" | "settings" | "export"
/** Panel sizes of one resizable group, as percentages by panel id. */
export type PanelSizes = Record<string, number>

type UiState = {
  theme: Theme
  keymap: KeymapPreset
  panels: Record<SidePanel, boolean>
  centerTab: CenterTab
  /** Saved sizes, keyed by group and by which panels were showing. */
  layouts: Record<string, PanelSizes>
  /** Bumped by "Reset layout" so the panel groups start over. */
  layoutGeneration: number
  selectedChannel: ChannelId | null
  selectedTrack: TrackId | null
  dialog: AppDialog | null

  setTheme(theme: Theme): void
  toggleTheme(): void
  setKeymap(keymap: KeymapPreset): void
  togglePanel(panel: SidePanel): void
  setPanelVisible(panel: SidePanel, visible: boolean): void
  showCenterTab(tab: CenterTab): void
  saveLayout(key: string, sizes: PanelSizes): void
  resetLayout(): void
  selectChannel(id: ChannelId | null): void
  selectTrack(id: TrackId | null): void
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
      layouts: {},
      layoutGeneration: 0,
      selectedChannel: null,
      selectedTrack: null,
      dialog: null,

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
      showCenterTab: (centerTab) => set({ centerTab }),
      saveLayout: (key, sizes) =>
        set((state) => ({ layouts: { ...state.layouts, [key]: sizes } })),
      resetLayout: () =>
        set((state) => ({
          panels: DEFAULT_PANELS,
          centerTab: "channelRack",
          layouts: {},
          layoutGeneration: state.layoutGeneration + 1,
        })),
      selectChannel: (selectedChannel) => set({ selectedChannel }),
      selectTrack: (selectedTrack) => set({ selectedTrack }),
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
