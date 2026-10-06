import type { Project } from "@/bindings"
import { registerBuiltinActions } from "@/lib/actions/builtin"
import { setBackend, type Backend } from "@/lib/ipc"
import { createMockBackend, type MockDialogs } from "@/lib/ipc/mock"
import { connectStores } from "@/lib/store/connect"
import { useEngineStore } from "@/lib/store/engine"
import { useProjectStore } from "@/lib/store/project"
import { usePromptStore } from "@/lib/store/prompts"
import { useTransportStore } from "@/lib/store/transport"
import { useUiStore } from "@/lib/store/ui"

export const TEST_DIALOGS: MockDialogs = {
  openProject: (saved) => Promise.resolve(saved[0] ?? null),
  saveProject: (suggested) => Promise.resolve(suggested),
  exportPath: (suggested) => Promise.resolve(suggested),
  folder: () => Promise.resolve("/samples/Test"),
}

/** Lets promises that are already resolved run their callbacks. */
export async function settle() {
  for (let turn = 0; turn < 10; turn += 1) await Promise.resolve()
}

/**
 * A fresh mock backend with the stores and actions wired to it, the way the
 * app starts. Call the returned `stop` when the test is done.
 */
export async function startTestApp(options: { project?: Project } = {}) {
  const backend: Backend = createMockBackend({
    storage: null,
    dialogs: TEST_DIALOGS,
    project: options.project,
  })
  setBackend(backend)
  useProjectStore.setState(useProjectStore.getInitialState(), true)
  useTransportStore.setState(useTransportStore.getInitialState(), true)
  useEngineStore.setState(useEngineStore.getInitialState(), true)
  useUiStore.setState(useUiStore.getInitialState(), true)
  usePromptStore.setState(usePromptStore.getInitialState(), true)
  const disconnect = connectStores()
  const unregister = registerBuiltinActions()
  await settle()
  return {
    backend,
    stop() {
      disconnect()
      unregister()
    },
  }
}
