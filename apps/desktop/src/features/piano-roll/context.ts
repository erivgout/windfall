import { createContext, useContext } from "react"

import type { PianoRollSession } from "./session"

export const SessionContext = createContext<PianoRollSession | null>(null)

/** The open piano roll's session. Only for components inside the panel. */
export function useSession(): PianoRollSession {
  const session = useContext(SessionContext)
  if (!session) throw new Error("useSession needs a piano roll around it")
  return session
}
