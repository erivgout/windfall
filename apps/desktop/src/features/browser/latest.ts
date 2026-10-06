export type LatestRunner<T> = {
  /** Asks for a job. It replaces any job still waiting its turn. */
  request(job: T): void
  /** Forgets the waiting job. A job already running is not interrupted. */
  cancel(): void
}

/**
 * Runs jobs one at a time and keeps only the newest one waiting. Holding an
 * arrow key in the browser asks for dozens of previews a second; this sends
 * the backend one call at a time, in order, and skips everything in between,
 * so a slow call can never land after a newer one and nothing piles up.
 *
 * `run` gets a `superseded` check that turns true once a newer job is
 * waiting. It should report its own failures; a rejection is ignored here.
 */
export function createLatestRunner<T>(
  run: (job: T, superseded: () => boolean) => Promise<void>
): LatestRunner<T> {
  let waiting: { job: T } | null = null
  let running = false

  async function drain() {
    running = true
    while (waiting !== null) {
      const { job } = waiting
      waiting = null
      try {
        await run(job, () => waiting !== null)
      } catch {
        // `run` has already shown what went wrong.
      }
    }
    running = false
  }

  return {
    request(job) {
      waiting = { job }
      // The first job starts in this same call, so a lone request is instant.
      if (!running) void drain()
    },
    cancel() {
      waiting = null
    },
  }
}
