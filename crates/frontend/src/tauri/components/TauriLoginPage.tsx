import { openOAuthLogin } from "../auth"

interface Props {
  onError: (error: string) => void
}

export function TauriLoginPage({ onError }: Props) {
  const handleLogin = async (provider: "google" | "apple") => {
    try {
      await openOAuthLogin(provider)
    } catch (e) {
      onError(`Failed to open login: ${e}`)
    }
  }

  return (
    <div className="tauri-login-page">
      <div className="login-container">
        <h1>Sign in to CalendSync</h1>
        <p>Choose a sign-in method to continue</p>
        <div className="login-buttons">
          <button
            type="button"
            onClick={() => handleLogin("google")}
            className="login-button google"
          >
            Sign in with Google
          </button>
          <button type="button" onClick={() => handleLogin("apple")} className="login-button apple">
            Sign in with Apple
          </button>
        </div>
      </div>
    </div>
  )
}
