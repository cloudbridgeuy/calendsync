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
        <link rel="stylesheet" href="/dist/calendsync.css" />
      </head>
      <body>
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
      </body>
    </html>
  )
}
