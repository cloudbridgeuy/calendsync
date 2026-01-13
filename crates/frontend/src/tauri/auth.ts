/**
 * Tauri authentication utilities.
 *
 * Provides functions for OAuth login via Tauri IPC.
 */

import { invoke } from "@tauri-apps/api/core"

/**
 * Open the system browser to initiate OAuth login with the specified provider.
 *
 * @param provider - The OAuth provider name ("google" or "apple")
 */
export async function openOAuthLogin(provider: "google" | "apple"): Promise<void> {
  return invoke("open_oauth_login", { provider })
}
