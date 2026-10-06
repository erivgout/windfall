import { create } from "zustand"

export type ConfirmChoice = {
  id: string
  label: string
  variant?: "default" | "outline" | "destructive"
}

export type ConfirmRequest = {
  title: string
  description: string
  /** Buttons, the main one last. A cancel button is always added first. */
  choices: ConfirmChoice[]
  cancelLabel?: string
}

export type TextRequest = {
  title: string
  description?: string
  label: string
  initial: string
  submitLabel: string
}

type PromptState = {
  confirm: (ConfirmRequest & { resolve(choice: string | null): void }) | null
  text: (TextRequest & { resolve(value: string | null): void }) | null
}

/** Questions waiting for an answer. `PromptHost` renders them as dialogs. */
export const usePromptStore = create<PromptState>(() => ({
  confirm: null,
  text: null,
}))

/** Asks a question. Resolves to the chosen id, or null when cancelled. */
export function askConfirm(request: ConfirmRequest): Promise<string | null> {
  return new Promise((resolve) => {
    usePromptStore.getState().confirm?.resolve(null)
    usePromptStore.setState({
      confirm: {
        ...request,
        resolve(choice) {
          usePromptStore.setState({ confirm: null })
          resolve(choice)
        },
      },
    })
  })
}

/** Asks for a line of text. Resolves to it, or null when cancelled. */
export function askText(request: TextRequest): Promise<string | null> {
  return new Promise((resolve) => {
    usePromptStore.getState().text?.resolve(null)
    usePromptStore.setState({
      text: {
        ...request,
        resolve(value) {
          usePromptStore.setState({ text: null })
          resolve(value)
        },
      },
    })
  })
}
