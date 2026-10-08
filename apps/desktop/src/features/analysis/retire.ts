import type { AnalysisBackend } from "./types"

const terminal = new Set(["cancelled", "failed", "consumed"])
/** Bounded control retirement; refusal retains native quota/evidence and finals.
 * There is no static job map or speculative filesystem deletion in the UI.
 */
export async function retireJob(
  api: AnalysisBackend,
  job: string
): Promise<void> {
  for (let attempt = 0; attempt < 600; attempt += 1) {
    const current = await api.analysisStatus(job)
    if (terminal.has(current.status)) {
      await api.analysisForget(job)
      return
    }
    await api.analysisCancel(job).catch(() => {}) // an apply claim can still own it
    const cancelled = await api.analysisStatus(job)
    if (terminal.has(cancelled.status)) {
      await api.analysisForget(job)
      return
    }
    await new Promise<void>((resolve) => setTimeout(resolve, 200))
  }
  throw new Error(
    `Analysis job ${job} remains retained. Use native status and cleanup retry; no files were deleted by this panel.`
  )
}
