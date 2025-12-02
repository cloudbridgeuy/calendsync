import { Counter } from "./components/Counter"
import { Greeting } from "./components/Greeting"
import { Weather, type WeatherData } from "./components/Weather"

interface AppProps {
    weather?: WeatherData | null
    weatherError?: string
    greeting: string
}

export function App({ weather, weatherError, greeting }: AppProps) {
    const tempC = weather?.tempC ?? 0

    return (
        <html lang="en">
            <head>
                <meta charSet="utf-8" />
                <title>Hello World - React SSR</title>
            </head>
            <body>
                <Greeting name="World" adjective={greeting} />
                <p>This was rendered server-side with React and Deno!</p>
                <hr />
                <Weather data={weather ?? null} error={weatherError} />
                <hr />
                <Counter initialValue={tempC} />

                {/* Embed initial state for hydration */}
                <script
                    dangerouslySetInnerHTML={{
                        __html: `window.__INITIAL_STATE__ = ${JSON.stringify({ weather, weatherError, greeting })}`,
                    }}
                />
                {/* Load client bundle for hydration */}
                <script src="/hello-world-client.js" />
            </body>
        </html>
    )
}
