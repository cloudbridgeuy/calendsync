import { SyncEngineProvider } from "@calendsync/contexts"
import { createTauriTransport, TransportProvider } from "@core/transport"
import { createRoot } from "react-dom/client"
import { App } from "./App"

const transport = createTauriTransport()

const root = document.getElementById("root")
if (!root) {
  throw new Error("Root element not found")
}

createRoot(root).render(
  <TransportProvider transport={transport}>
    <SyncEngineProvider>
      <App />
    </SyncEngineProvider>
  </TransportProvider>,
)
