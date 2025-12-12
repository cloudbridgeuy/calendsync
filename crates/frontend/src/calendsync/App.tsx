/**
 * Root App component for the React Calendar.
 * Renders the full HTML document for SSR.
 */

import { Calendar } from "./components"
import type { InitialData } from "./types"

interface AppProps {
  initialData: InitialData
}

/**
 * Root application component.
 * Renders complete HTML document for SSR with hydration support.
 */
export function App({ initialData }: AppProps) {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Calendar - CalendSync</title>
        <link rel="stylesheet" href={initialData.cssBundleUrl ?? "/dist/calendsync.css"} />
      </head>
      <body>
        {/* Dev mode indicator badge */}
        {initialData.devMode && (
          <div
            style={{
              position: "fixed",
              top: "8px",
              right: "8px",
              background: "#ef4444",
              color: "white",
              padding: "4px 8px",
              borderRadius: "4px",
              fontSize: "12px",
              fontWeight: "bold",
              zIndex: 9999,
              fontFamily: "system-ui, sans-serif",
            }}
          >
            DEV
          </div>
        )}

        <Calendar initialData={initialData}>
          <Calendar.Header />
          <Calendar.NotificationCenter />
          <Calendar.Days />
          <Calendar.TodayButton />
          <Calendar.Fab />
          <Calendar.Modal />
        </Calendar>

        {/* Embed initial state for hydration */}
        <script
          // biome-ignore lint/security/noDangerouslySetInnerHtml: Required for SSR hydration data
          dangerouslySetInnerHTML={{
            __html: `window.__INITIAL_DATA__ = ${JSON.stringify(initialData)};`,
          }}
        />

        {/* Load client bundle for hydration */}
        <script type="module" src={initialData.clientBundleUrl} />

        {/* Dev mode auto-refresh: connect to SSE and reload on signal */}
        {initialData.devMode && (
          <script
            // biome-ignore lint/security/noDangerouslySetInnerHtml: Dev mode auto-refresh script
            dangerouslySetInnerHTML={{
              __html: `
(function() {
  var es = new EventSource('/_dev/events');
  es.addEventListener('reload', function() {
    console.log('[Dev] Reload signal received, refreshing...');
    location.reload();
  });
  es.addEventListener('connected', function() {
    console.log('[Dev] Auto-refresh connected');
  });
  es.onerror = function() {
    console.log('[Dev] Auto-refresh disconnected, will retry...');
  };
})();
              `.trim(),
            }}
          />
        )}
      </body>
    </html>
  )
}
