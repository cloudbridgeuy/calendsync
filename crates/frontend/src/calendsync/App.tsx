/**
 * Root App component for the React Calendar.
 * Renders the full HTML document for SSR.
 */

import { Calendar, DevMenu } from "./components"
import { FlashMessage } from "./components/FlashMessage"
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
        {/* Flash message for server-to-client notifications */}
        <FlashMessage />

        {/* Dev mode menu with tools */}
        <DevMenu initialData={initialData} />

        <Calendar initialData={initialData}>
          <Calendar.Header />
          <Calendar.NotificationCenter />
          <Calendar.View />
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
  var errorOverlay = null;

  function showError(error) {
    // Remove existing overlay if any
    hideError();

    // Create overlay
    errorOverlay = document.createElement('div');
    errorOverlay.id = 'dev-error-overlay';
    errorOverlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.9);z-index:10000;overflow:auto;padding:20px;font-family:monospace;';

    // Header with dismiss button
    var header = document.createElement('div');
    header.style.cssText = 'display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;';

    var title = document.createElement('h2');
    title.textContent = 'Build Error';
    title.style.cssText = 'color:#ef4444;margin:0;font-size:20px;';

    var dismissBtn = document.createElement('button');
    dismissBtn.textContent = 'Dismiss';
    dismissBtn.style.cssText = 'background:#3b82f6;color:white;border:none;padding:8px 16px;border-radius:4px;cursor:pointer;font-size:14px;';
    dismissBtn.onclick = hideError;

    header.appendChild(title);
    header.appendChild(dismissBtn);

    // Error content
    var content = document.createElement('pre');
    content.textContent = error;
    content.style.cssText = 'color:#fca5a5;white-space:pre-wrap;word-break:break-word;font-size:14px;line-height:1.5;margin:0;';

    errorOverlay.appendChild(header);
    errorOverlay.appendChild(content);
    document.body.appendChild(errorOverlay);
  }

  function hideError() {
    if (errorOverlay) {
      errorOverlay.remove();
      errorOverlay = null;
    }
  }

  es.addEventListener('reload', function() {
    console.log('[Dev] Reload signal received, refreshing...');
    hideError(); // Clear any error overlay before reload
    location.reload();
  });

  es.addEventListener('css-reload', function(e) {
    try {
      var data = JSON.parse(e.data);
      var links = document.querySelectorAll('link[rel="stylesheet"]');
      for (var i = 0; i < links.length; i++) {
        var link = links[i];
        if (link.href && link.href.indexOf('calendsync') !== -1) {
          link.href = '/dist/' + data.filename;
          console.log('[Dev] CSS hot-swapped:', data.filename);
          break;
        }
      }
    } catch (err) {
      console.error('[Dev] Failed to parse CSS reload:', err);
    }
  });

  es.addEventListener('build-error', function(e) {
    try {
      var data = JSON.parse(e.data);
      console.log('[Dev] Build error received');
      showError(data.error);
    } catch (err) {
      console.error('[Dev] Failed to parse build error:', err);
    }
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
