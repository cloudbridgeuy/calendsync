/**
 * SettingsMenuWrapper - bridges CalendarContext to SettingsMenu component.
 * Extracts settings state and actions from the calendar context and passes them
 * to the SettingsMenu compound component.
 */

import { useTransport } from "@core/transport"
import { useCallback } from "react"
import { useCalendarContext } from "../contexts"
import { SettingsMenu } from "./SettingsMenu"

/**
 * SettingsMenuWrapper extracts settings from CalendarContext and renders
 * the SettingsMenu compound component with all required props and children.
 */
export function SettingsMenuWrapper() {
  const { settings, setViewMode, toggleShowTasks, setEntryStyle, user } = useCalendarContext()
  const transport = useTransport()

  const handleLogout = useCallback(async () => {
    await transport.logout()
    window.location.href = "/login"
  }, [transport])

  return (
    <SettingsMenu
      viewMode={settings.viewMode}
      showTasks={settings.showTasks}
      entryStyle={settings.entryStyle}
      user={user}
      onViewModeChange={setViewMode}
      onToggleShowTasks={toggleShowTasks}
      onEntryStyleChange={setEntryStyle}
      onLogout={handleLogout}
    >
      <SettingsMenu.Trigger />
      <SettingsMenu.Panel>
        <SettingsMenu.Profile />
        <SettingsMenu.ViewToggle />
        <SettingsMenu.StyleToggle />
        <SettingsMenu.TaskToggle />
      </SettingsMenu.Panel>
    </SettingsMenu>
  )
}
