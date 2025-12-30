/**
 * Settings management hook using Tauri Store plugin
 *
 * Provides direct access to persistent settings without backend commands.
 * All operations are wrapped in proper error handling to prevent silent failures.
 */

import { load } from '@tauri-apps/plugin-store';

const SETTINGS_FILE = 'settings.json';
let storeInstance: Awaited<ReturnType<typeof load>> | null = null;
let storePromise: Promise<Awaited<ReturnType<typeof load>>> | null = null;

/**
 * Initialize the settings store (lazy loading with concurrent access protection)
 *
 * This function prevents race conditions when multiple components
 * try to access the store simultaneously by caching the initialization promise.
 *
 * @returns Promise resolving to the store instance
 * @throws Error if store initialization fails
 */
async function getStore() {
  if (storeInstance) {
    return storeInstance;
  }

  if (!storePromise) {
    storePromise = load(SETTINGS_FILE, { autoSave: true, defaults: {} })
      .then(store => {
        storeInstance = store;
        storePromise = null;
        return store;
      })
      .catch(error => {
        storePromise = null;
        throw new Error(`Failed to initialize settings store: ${error}`);
      });
  }

  return storePromise;
}

/**
 * Get the current theme preference
 *
 * @returns Promise resolving to theme value ('auto', 'light', or 'dark')
 * @throws Error if store access fails
 */
export async function getTheme(): Promise<string> {
  try {
    const store = await getStore();
    return (await store.get<string>('theme')) || 'auto';
  } catch (error) {
    throw new Error(`Failed to get theme: ${error}`);
  }
}

/**
 * Set the theme preference
 *
 * @param theme - Theme value ('auto', 'light', or 'dark')
 * @throws Error if store access or save fails
 */
export async function setTheme(theme: string): Promise<void> {
  try {
    const store = await getStore();
    await store.set('theme', theme);
    await store.save(); // Explicitly save to ensure persistence
  } catch (error) {
    throw new Error(`Failed to set theme: ${error}`);
  }
}

/**
 * Get the current language preference
 *
 * @returns Promise resolving to language code (e.g., 'en', 'de', 'fr')
 * @throws Error if store access fails
 */
export async function getLanguage(): Promise<string> {
  try {
    const store = await getStore();
    return (await store.get<string>('language')) || 'en';
  } catch (error) {
    throw new Error(`Failed to get language: ${error}`);
  }
}

/**
 * Set the language preference
 *
 * @param language - Language code (e.g., 'en', 'de', 'fr')
 * @throws Error if store access or save fails
 */
export async function setLanguage(language: string): Promise<void> {
  try {
    const store = await getStore();
    await store.set('language', language);
    await store.save(); // Explicitly save to ensure persistence
  } catch (error) {
    throw new Error(`Failed to set language: ${error}`);
  }
}

/**
 * Get the MOTD visibility preference
 *
 * @returns Promise resolving to true if MOTD should be shown, false otherwise
 * @throws Error if store access fails
 */
export async function getShowMotd(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>('show_motd');
    return value ?? true; // Default to true if not set
  } catch (error) {
    throw new Error(`Failed to get MOTD preference: ${error}`);
  }
}

/**
 * Set the MOTD visibility preference
 *
 * @param show - true to show MOTD, false to hide
 * @throws Error if store access or save fails
 */
export async function setShowMotd(show: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set('show_motd', show);
    await store.save(); // Explicitly save to ensure persistence
  } catch (error) {
    throw new Error(`Failed to set MOTD preference: ${error}`);
  }
}

/**
 * Get the updater modal visibility preference
 *
 * @returns Promise resolving to true if updater modal should be shown, false otherwise
 * @throws Error if store access fails
 */
export async function getShowUpdaterModal(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>('show_updater_modal');
    return value ?? true; // Default to true if not set
  } catch (error) {
    throw new Error(`Failed to get updater modal preference: ${error}`);
  }
}

/**
 * Set the updater modal visibility preference
 *
 * @param show - true to show updater modal, false to hide
 * @throws Error if store access or save fails
 */
export async function setShowUpdaterModal(show: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set('show_updater_modal', show);
    await store.save(); // Explicitly save to ensure persistence
  } catch (error) {
    throw new Error(`Failed to set updater modal preference: ${error}`);
  }
}

/**
 * Get the developer mode preference
 *
 * @returns Promise resolving to true if developer mode is enabled, false otherwise
 * @throws Error if store access fails
 */
export async function getDeveloperMode(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>('developer_mode');
    return value ?? false; // Default to false if not set
  } catch (error) {
    throw new Error(`Failed to get developer mode preference: ${error}`);
  }
}

/**
 * Set the developer mode preference
 *
 * This setting controls debug logging verbosity across the application.
 * When enabled, more detailed debug information is logged.
 *
 * @param enabled - true to enable developer mode, false to disable
 * @throws Error if store access or save fails
 */
export async function setDeveloperMode(enabled: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set('developer_mode', enabled);
    await store.save(); // Explicitly save to ensure persistence
  } catch (error) {
    throw new Error(`Failed to set developer mode preference: ${error}`);
  }
}
