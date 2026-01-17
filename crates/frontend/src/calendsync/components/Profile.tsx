/**
 * Profile component - displays user info and logout button.
 * Used within the SettingsMenu as a standalone component.
 */

import type { UserInfo } from "../types"

export interface ProfileProps {
  /** User information to display */
  user?: UserInfo
  /** Callback when logout is clicked */
  onLogout: () => void
}

/**
 * Profile displays the logged-in user's name, email, and a logout button.
 * Returns null if no user is provided (not logged in).
 */
export function Profile({ user, onLogout }: ProfileProps) {
  if (!user) return null

  return (
    <div className="settings-profile">
      <div className="settings-profile-info">
        <span className="settings-profile-name">{user.name}</span>
        <span className="settings-profile-email">{user.email}</span>
      </div>
      <button type="button" className="settings-logout-button" onClick={onLogout}>
        Log out
      </button>
    </div>
  )
}
