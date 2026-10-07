import { sim } from "@/lib/ipc/sim/wasm"
import type { SliceAnalysis, SliceAnalysisRequest } from "./types"

self.onmessage = (event: MessageEvent<SliceAnalysisRequest>) => {
  try {
    self.postMessage({
      analysis: sim.call<SliceAnalysis>("slice_analyze", 0, event.data),
    })
  } catch (error) {
    self.postMessage({
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
self.postMessage({ ready: true })
