// Weather data structure from wttr.in API
interface WeatherData {
    location: string
    temperature: string
    condition: string
    humidity: string
    wind: string
    tempC: number // Raw temperature in Celsius for Counter
}

interface WeatherProps {
    data: WeatherData | null
    error?: string
}

export function Weather({ data, error }: WeatherProps) {
    if (error) {
        return (
            <div className="weather weather-error">
                <h2>Weather</h2>
                <p>Failed to load weather: {error}</p>
            </div>
        )
    }

    if (!data) {
        return (
            <div className="weather weather-loading">
                <h2>Weather</h2>
                <p>Loading weather data...</p>
            </div>
        )
    }

    return (
        <div className="weather">
            <h2>Weather in {data.location}</h2>
            <ul>
                <li>Temperature: {data.temperature}</li>
                <li>Condition: {data.condition}</li>
                <li>Humidity: {data.humidity}</li>
                <li>Wind: {data.wind}</li>
            </ul>
        </div>
    )
}

export type { WeatherData }
