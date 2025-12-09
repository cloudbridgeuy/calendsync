/**
 * Browser capability detection for navigation feedback.
 * Pure functions that check browser API availability.
 */

/**
 * Check if the Vibration API is supported in the current browser.
 * @returns true if navigator.vibrate is available
 */
export function isVibrationSupported(): boolean {
  return typeof navigator !== "undefined" && "vibrate" in navigator
}

/**
 * Check if the Web Audio API is supported in the current browser.
 * @returns true if AudioContext is available
 */
export function isAudioSupported(): boolean {
  return (
    typeof window !== "undefined" && ("AudioContext" in window || "webkitAudioContext" in window)
  )
}
