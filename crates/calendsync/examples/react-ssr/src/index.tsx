import { faker } from "@faker-js/faker"
import { renderToString } from "react-dom/server"
import { App } from "./App"
import type { WeatherData } from "./components/Weather"

// Declare the custom ops provided by Rust
declare const Deno: {
    core: {
        ops: {
            op_set_html(html: string): void
            op_fetch(url: string): Promise<string>
        }
    }
}

// Declare the SSR config provided by Rust
declare const __SSR_CONFIG__: { weatherApiUrl: string }

async function fetchWeather(city: string): Promise<WeatherData> {
    console.log(`[SSR] Fetching weather for ${city}...`)
    const response = await fetch(__SSR_CONFIG__.weatherApiUrl)
    const data = await response.json()

    const current = data.current_condition[0]
    const location = data.nearest_area[0]

    return {
        location: `${location.areaName[0].value}, ${location.country[0].value}`,
        temperature: `${current.temp_C}°C (${current.temp_F}°F)`,
        condition: current.weatherDesc[0].value,
        humidity: `${current.humidity}%`,
        wind: `${current.windspeedKmph} km/h ${current.winddir16Point}`,
        tempC: Number.parseInt(current.temp_C, 10),
    }
}

async function main() {
    console.log("[SSR] Starting React SSR...")

    let weather: WeatherData | null = null
    let weatherError: string | undefined

    try {
        weather = await fetchWeather("London")
        console.log("[SSR] Weather data fetched successfully")
    } catch (err) {
        weatherError = err instanceof Error ? err.message : String(err)
        console.log(`[SSR] Weather fetch error: ${weatherError}`)
    }

    // Generate random adjective once during SSR - passed to client via __INITIAL_STATE__
    const greeting = faker.word.adjective()
    console.log(`[SSR] Generated greeting adjective: ${greeting}`)

    const html = renderToString(
        <App weather={weather} weatherError={weatherError} greeting={greeting} />,
    )
    console.log("[SSR] React render complete")

    Deno.core.ops.op_set_html(html)
}

main()
