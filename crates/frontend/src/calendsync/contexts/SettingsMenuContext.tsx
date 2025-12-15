/**
 * SettingsMenuContext - provides shared state to SettingsMenu sub-components.
 * Enables the compound component pattern for SettingsMenu.
 */

import { buildAriaIds } from "@core/calendar"
import type { ViewMode } from "@core/calendar/settings"
import { createContext, useCallback, useContext, useId, useMemo, useState } from "react"

/** Settings menu state */
export interface SettingsMenuState {
  /** Whether the settings panel is open */
  isOpen: boolean
  /** Current view mode */
  viewMode: ViewMode
  /** Whether to show task entries */
  showTasks: boolean
}

/** Settings menu actions */
export interface SettingsMenuActions {
  /** Toggle the panel open/closed */
  toggleOpen: () => void
  /** Close the panel */
  close: () => void
  /** Set the view mode */
  setViewMode: (mode: ViewMode) => void
  /** Toggle task visibility */
  toggleShowTasks: () => void
}

/** Context value shared with settings menu sub-components */
export interface SettingsMenuContextValue {
  /** Current state */
  state: SettingsMenuState
  /** Available actions */
  actions: SettingsMenuActions
  /** ARIA ID for the trigger button */
  triggerId: string
  /** ARIA ID for the panel */
  contentId: string
  /** Ref callback for the panel element */
  panelRef: React.RefCallback<HTMLDivElement>
  /** Ref callback for the trigger button element */
  buttonRef: React.RefCallback<HTMLButtonElement>
}

/** SettingsMenuContext - null when not inside provider */
const SettingsMenuContext = createContext<SettingsMenuContextValue | null>(null)

/** Props for SettingsMenuProvider */
export interface SettingsMenuProviderProps {
  children: React.ReactNode
  /** Current view mode from calendar settings */
  viewMode: ViewMode
  /** Current showTasks from calendar settings */
  showTasks: boolean
  /** Callback to set view mode */
  onViewModeChange: (mode: ViewMode) => void
  /** Callback to toggle showTasks */
  onToggleShowTasks: () => void
}

/**
 * SettingsMenuProvider - wraps settings menu sub-components with shared context.
 */
export function SettingsMenuProvider({
  children,
  viewMode,
  showTasks,
  onViewModeChange,
  onToggleShowTasks,
}: SettingsMenuProviderProps) {
  const id = useId()
  const { triggerId, contentId } = buildAriaIds(`settings-menu-${id}`)
  const [isOpen, setIsOpen] = useState(false)

  const toggleOpen = useCallback(() => {
    setIsOpen((prev) => !prev)
  }, [])

  const close = useCallback(() => {
    setIsOpen(false)
  }, [])

  const setViewMode = useCallback(
    (mode: ViewMode) => {
      onViewModeChange(mode)
    },
    [onViewModeChange],
  )

  const toggleShowTasks = useCallback(() => {
    onToggleShowTasks()
  }, [onToggleShowTasks])

  // Ref callbacks for sub-components
  const panelRef = useCallback((_node: HTMLDivElement | null) => {
    // Callback provided for future use (e.g., click-outside detection)
  }, [])

  const buttonRef = useCallback((_node: HTMLButtonElement | null) => {
    // Callback provided for future use
  }, [])

  const state: SettingsMenuState = useMemo(
    () => ({
      isOpen,
      viewMode,
      showTasks,
    }),
    [isOpen, viewMode, showTasks],
  )

  const actions: SettingsMenuActions = useMemo(
    () => ({
      toggleOpen,
      close,
      setViewMode,
      toggleShowTasks,
    }),
    [toggleOpen, close, setViewMode, toggleShowTasks],
  )

  const value = useMemo<SettingsMenuContextValue>(
    () => ({
      state,
      actions,
      triggerId,
      contentId,
      panelRef,
      buttonRef,
    }),
    [state, actions, triggerId, contentId, panelRef, buttonRef],
  )

  return <SettingsMenuContext.Provider value={value}>{children}</SettingsMenuContext.Provider>
}

/**
 * Hook to access SettingsMenuContext.
 * Throws if used outside SettingsMenuProvider.
 */
export function useSettingsMenuContext(): SettingsMenuContextValue {
  const ctx = useContext(SettingsMenuContext)
  if (!ctx) {
    throw new Error("useSettingsMenuContext must be used within SettingsMenuProvider")
  }
  return ctx
}
