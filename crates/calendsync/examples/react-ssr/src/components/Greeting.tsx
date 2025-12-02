interface GreetingProps {
    name: string
    adjective: string
}

// Adjective is generated once during SSR and passed via props to ensure hydration matches
export function Greeting({ name, adjective }: GreetingProps) {
    return (
        <h1>
            Hello, {adjective} {name}!
        </h1>
    )
}
