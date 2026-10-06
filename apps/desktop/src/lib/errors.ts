import { toast } from "sonner"

import { errorMessage } from "@/lib/ipc/backend"

/**
 * Shows a failed action to the user. Every rejected backend call ends up
 * here, so nothing fails silently.
 */
export function reportError(error: unknown, what?: string) {
  const message = errorMessage(error)
  if (what) toast.error(what, { description: message })
  else toast.error(message)
}

/** Awaits a backend call. A failure is shown and turned into `null`. */
export async function attempt<T>(
  work: Promise<T>,
  what?: string
): Promise<T | null> {
  try {
    return await work
  } catch (error) {
    reportError(error, what)
    return null
  }
}
