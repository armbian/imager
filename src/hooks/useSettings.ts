/**
 * Settings management hook using Tauri Store plugin
 *
 * Provides direct access to persistent settings without backend commands.
 * All operations are wrapped in proper error handling to prevent silent failures.
 */

import { load } from '@tauri-apps/plugin-store';
import { CACHE, SETTINGS } from '../config';
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
    storePromise = load(SETTINGS.FILE, { autoSave: true, defaults: {} })
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
    return (await store.get<string>(SETTINGS.KEYS.THEME)) || SETTINGS.DEFAULTS.THEME;
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
    await store.set(SETTINGS.KEYS.THEME, theme);
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
    return (await store.get<string>(SETTINGS.KEYS.LANGUAGE)) || SETTINGS.DEFAULTS.LANGUAGE;
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
    await store.set(SETTINGS.KEYS.LANGUAGE, language);
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
    const value = await store.get<boolean>(SETTINGS.KEYS.SHOW_MOTD);
    return value ?? SETTINGS.DEFAULTS.SHOW_MOTD;
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
    await store.set(SETTINGS.KEYS.SHOW_MOTD, show);
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
    const value = await store.get<boolean>(SETTINGS.KEYS.SHOW_UPDATER_MODAL);
    return value ?? SETTINGS.DEFAULTS.SHOW_UPDATER_MODAL;
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
    await store.set(SETTINGS.KEYS.SHOW_UPDATER_MODAL, show);
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
    const value = await store.get<boolean>(SETTINGS.KEYS.DEVELOPER_MODE);
    return value ?? SETTINGS.DEFAULTS.DEVELOPER_MODE;
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
    await store.set(SETTINGS.KEYS.DEVELOPER_MODE, enabled);
    await store.save(); // Explicitly save to ensure persistence
  } catch (error) {
    throw new Error(`Failed to set developer mode preference: ${error}`);
  }
}

// ============================================================================
// Cache Settings
// ============================================================================

// Note: The canonical default cache size is defined in the Rust backend
// (src-tauri/src/config/mod.rs). This fallback is only used if the
// backend value cannot be retrieved. The backend is the source of truth.

/**
 * Get the cache enabled preference
 *
 * @returns Promise resolving to true if image caching is enabled, false otherwise
 * @throws Error if store access fails
 */
export async function getCacheEnabled(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.CACHE_ENABLED);
    return value ?? SETTINGS.DEFAULTS.CACHE_ENABLED;
  } catch (error) {
    throw new Error(`Failed to get cache enabled preference: ${error}`);
  }
}

/**
 * Set the cache enabled preference
 *
 * When enabled, downloaded images are kept for faster retry if flashing fails.
 * When disabled, images are deleted after successful flash.
 *
 * @param enabled - true to enable image caching, false to disable
 * @throws Error if store access or save fails
 */
export async function setCacheEnabled(enabled: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.CACHE_ENABLED, enabled);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set cache enabled preference: ${error}`);
  }
}

/**
 * Get the maximum cache size in bytes
 *
 * If no value is stored, returns null to indicate the backend default should be used.
 * The caller should handle null by using the backend's default value.
 *
 * @returns Promise resolving to maximum cache size in bytes, or null if not set
 * @throws Error if store access fails
 */
export async function getCacheMaxSize(): Promise<number> {
  try {
    const store = await getStore();
    const value = await store.get<number>(SETTINGS.KEYS.CACHE_MAX_SIZE);
    // Return a reasonable fallback if not set - matches backend default
    // This is only a fallback; the backend is the source of truth
    return value ?? CACHE.DEFAULT_SIZE;
  } catch (error) {
    throw new Error(`Failed to get cache max size: ${error}`);
  }
}

/**
 * Set the maximum cache size in bytes
 *
 * When the cache exceeds this size, oldest images are automatically removed.
 *
 * @param size - Maximum cache size in bytes
 * @throws Error if store access or save fails
 */
export async function setCacheMaxSize(size: number): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.CACHE_MAX_SIZE, size);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set cache max size: ${error}`);
  }
}
