import { lazy, StrictMode, Suspense } from "react"
import { createRoot } from "react-dom/client"

import "./index.css"
import App from "./App.tsx"
import { ThemeProvider } from "@/components/theme-provider.tsx"

// `?view=kit` opens the audio control kit showcase instead of the app.
const view = new URLSearchParams(window.location.search).get("view")
const KitDemo = lazy(() => import("@/features/kit-demo"))

// The stylesheet locks scrolling and text selection for the app window only.
document.documentElement.dataset.view = view === "kit" ? "kit" : "app"

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      {view === "kit" ? (
        <Suspense>
          <KitDemo />
        </Suspense>
      ) : (
        <App />
      )}
    </ThemeProvider>
  </StrictMode>
)
