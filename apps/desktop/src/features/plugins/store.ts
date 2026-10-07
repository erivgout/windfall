import { create } from "zustand"
import type { PluginBinding, PluginTarget, TrackId } from "@/bindings"
import { registry } from "@/lib/actions/registry"
import { useProjectStore } from "@/lib/store"

export const usePluginUi = create<{ open: boolean; track?: TrackId }>(() => ({
  open: false,
}))
export function openPluginManager(track?: TrackId) {
  usePluginUi.setState({ open: true, track })
}
export function usePluginBinding(
  target: PluginTarget
): PluginBinding | undefined {
  return useProjectStore((state) =>
    (state.project.plugins ?? []).find((plugin) =>
      sameTarget(plugin.target, target)
    )
  )
}
export function sameTarget(a: PluginTarget, b: PluginTarget): boolean {
  return a.type === "instrument"
    ? b.type === "instrument" && a.channel === b.channel
    : b.type === "effect" && a.effect === b.effect
}
export function registerPluginActions() {
  return registry.register([
    {
      id: "plugins.manage",
      title: "Plugin manager",
      section: "Options",
      keywords: "CLAP scan instruments effects native",
      run: () => openPluginManager(),
    },
  ])
}
