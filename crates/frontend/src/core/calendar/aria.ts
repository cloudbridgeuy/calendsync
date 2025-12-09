/**
 * ARIA utilities - pure functions for accessibility support.
 * No side effects - these are part of the Functional Core.
 */

/** ARIA ID pair for accessible components */
export interface AriaIds {
  triggerId: string;
  contentId: string;
}

/**
 * Build ARIA IDs for coordinated trigger/content components.
 * Used for dropdown panels, modals, and other disclosure patterns.
 */
export function buildAriaIds(baseId: string): AriaIds {
  return {
    triggerId: `${baseId}-trigger`,
    contentId: `${baseId}-content`,
  };
}
