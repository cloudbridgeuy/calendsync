import { useState } from "react"

interface CounterProps {
    initialValue: number
}

export function Counter({ initialValue }: CounterProps) {
    const [count, setCount] = useState(initialValue)

    return (
        <div className="counter">
            <h2>Counter (initialized from temperature)</h2>
            <p>Count: {count}</p>
            <button type="button" onClick={() => setCount((c) => c - 1)}>
                -
            </button>
            <button type="button" onClick={() => setCount((c) => c + 1)}>
                +
            </button>
        </div>
    )
}
