import { hydrateRoot } from "react-dom/client"
import { App } from "./App"
import type { WeatherData } from "./components/Weather"

// Declare the global state that was embedded by the server
declare global {
    interface Window {
        __INITIAL_STATE__: {
            weather: WeatherData | null
            weatherError?: string
            greeting: string
        }
    }
}

// Read initial state from the server-embedded script
const { weather, weatherError, greeting } = window.__INITIAL_STATE__

console.log("[Client] Hydrating React app...")

// Hydrate the React tree - attaches event handlers to existing DOM
hydrateRoot(document, <App weather={weather} weatherError={weatherError} greeting={greeting} />)

console.log("[Client] Hydration complete!")
