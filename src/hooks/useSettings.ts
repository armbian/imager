// Persistent settings access via the Tauri Store plugin (no backend commands)

import { load } from '@tauri-apps/plugin-store';
import { CACHE, EVENTS, SETTINGS } from '../config';
import type { AutoconfigProfile, AutoconfigProfilesChangedDetail } from '../types';
let storeInstance: Awaited<ReturnType<typeof load>> | null = null;
let storePromise: Promise<Awaited<ReturnType<typeof load>>> | null = null;

// Lazily load the store, caching the in-flight promise to avoid concurrent re-init
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

/** Get the theme preference ('auto', 'light', or 'dark') */
export async function getTheme(): Promise<string> {
  try {
    const store = await getStore();
    return (await store.get<string>(SETTINGS.KEYS.THEME)) || SETTINGS.DEFAULTS.THEME;
  } catch (error) {
    throw new Error(`Failed to get theme: ${error}`);
  }
}

/** Set the theme preference ('auto', 'light', or 'dark') */
export async function setTheme(theme: string): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.THEME, theme);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set theme: ${error}`);
  }
}

/** Get the MOTD visibility preference */
export async function getShowMotd(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.SHOW_MOTD);
    return value ?? SETTINGS.DEFAULTS.SHOW_MOTD;
  } catch (error) {
    throw new Error(`Failed to get MOTD preference: ${error}`);
  }
}

/** Set the MOTD visibility preference */
export async function setShowMotd(show: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.SHOW_MOTD, show);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set MOTD preference: ${error}`);
  }
}

/** Get the welcome screen visibility preference */
export async function getShowWelcome(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.SHOW_WELCOME);
    return value ?? SETTINGS.DEFAULTS.SHOW_WELCOME;
  } catch (error) {
    throw new Error(`Failed to get welcome screen preference: ${error}`);
  }
}

/** Set the welcome screen visibility preference */
export async function setShowWelcome(show: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.SHOW_WELCOME, show);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set welcome screen preference: ${error}`);
  }
}

/** Get the updater modal visibility preference */
export async function getShowUpdaterModal(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.SHOW_UPDATER_MODAL);
    return value ?? SETTINGS.DEFAULTS.SHOW_UPDATER_MODAL;
  } catch (error) {
    throw new Error(`Failed to get updater modal preference: ${error}`);
  }
}

/** Set the updater modal visibility preference */
export async function setShowUpdaterModal(show: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.SHOW_UPDATER_MODAL, show);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set updater modal preference: ${error}`);
  }
}

/** Get the developer mode preference */
export async function getDeveloperMode(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.DEVELOPER_MODE);
    return value ?? SETTINGS.DEFAULTS.DEVELOPER_MODE;
  } catch (error) {
    throw new Error(`Failed to get developer mode preference: ${error}`);
  }
}

/** Set developer mode, which controls debug logging verbosity */
export async function setDeveloperMode(enabled: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.DEVELOPER_MODE, enabled);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set developer mode preference: ${error}`);
  }
}

/** Get the skip-verification preference */
export async function getSkipVerify(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.SKIP_VERIFY);
    return value ?? SETTINGS.DEFAULTS.SKIP_VERIFY;
  } catch (error) {
    throw new Error(`Failed to get skip verify preference: ${error}`);
  }
}

/** Set the skip-verification preference (skips the post-flash check for speed) */
export async function setSkipVerify(skip: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.SKIP_VERIFY, skip);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set skip verify preference: ${error}`);
  }
}

/** Get the force-offline preference: behave as if there's no connectivity */
export async function getForceOffline(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.FORCE_OFFLINE);
    return value ?? SETTINGS.DEFAULTS.FORCE_OFFLINE;
  } catch (error) {
    throw new Error(`Failed to get force offline preference: ${error}`);
  }
}

/** Set the force-offline preference: behave as if there's no connectivity */
export async function setForceOffline(value: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.FORCE_OFFLINE, value);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set force offline preference: ${error}`);
  }
}

/** Get the allow-system-devices preference: show and allow flashing internal/system disks */
export async function getAllowSystemDevices(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.ALLOW_SYSTEM_DEVICES);
    return value ?? SETTINGS.DEFAULTS.ALLOW_SYSTEM_DEVICES;
  } catch (error) {
    throw new Error(`Failed to get allow system devices preference: ${error}`);
  }
}

/** Set the allow-system-devices preference: show and allow flashing internal/system disks */
export async function setAllowSystemDevices(value: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.ALLOW_SYSTEM_DEVICES, value);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set allow system devices preference: ${error}`);
  }
}

// Cache settings: backend owns the canonical defaults; values here are fallbacks when it's unreachable.

/** Get the cache enabled preference */
export async function getCacheEnabled(): Promise<boolean> {
  try {
    const store = await getStore();
    const value = await store.get<boolean>(SETTINGS.KEYS.CACHE_ENABLED);
    return value ?? SETTINGS.DEFAULTS.CACHE_ENABLED;
  } catch (error) {
    throw new Error(`Failed to get cache enabled preference: ${error}`);
  }
}

/** Set cache enabled; when on, downloaded images are kept for faster retry instead of deleted after flash */
export async function setCacheEnabled(enabled: boolean): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.CACHE_ENABLED, enabled);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set cache enabled preference: ${error}`);
  }
}

/** Get the maximum cache size in bytes, falling back to the backend default when unset */
export async function getCacheMaxSize(): Promise<number> {
  try {
    const store = await getStore();
    const value = await store.get<number>(SETTINGS.KEYS.CACHE_MAX_SIZE);
    return value ?? CACHE.DEFAULT_SIZE;
  } catch (error) {
    throw new Error(`Failed to get cache max size: ${error}`);
  }
}

/** Set the max cache size in bytes; older images are evicted past this limit */
export async function setCacheMaxSize(size: number): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.CACHE_MAX_SIZE, size);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set cache max size: ${error}`);
  }
}

/** Get the Armbian board detection mode: "disabled", "modal" (confirm before auto-select), or "auto" (no confirmation) */
export async function getArmbianBoardDetection(): Promise<string> {
  try {
    const store = await getStore();
    return (
      (await store.get<string>(SETTINGS.KEYS.ARMBIAN_BOARD_DETECTION)) ||
      SETTINGS.DEFAULTS.ARMBIAN_BOARD_DETECTION
    );
  } catch (error) {
    throw new Error(`Failed to get Armbian board detection preference: ${error}`);
  }
}

/** Set the Armbian board detection mode ('disabled', 'modal', or 'auto') */
export async function setArmbianBoardDetection(mode: string): Promise<void> {
  if (!['disabled', 'modal', 'auto'].includes(mode)) {
    throw new Error(
      `Invalid Armbian board detection mode: ${mode}. Must be 'disabled', 'modal', or 'auto'`
    );
  }

  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.ARMBIAN_BOARD_DETECTION, mode);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to set Armbian board detection preference: ${error}`);
  }
}

// Autoconfig profiles: named first-boot presets stored client-side and applied only on explicit selection.

/** Notify open views (settings, flash) which profile changed and how */
function emitProfilesChanged(detail: AutoconfigProfilesChangedDetail): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent(EVENTS.PROFILES_CHANGED, { detail }));
  }
}

/** Get all stored autoconfig profiles, or an empty list when none exist */
export async function getAutoconfigProfiles(): Promise<AutoconfigProfile[]> {
  try {
    const store = await getStore();
    const value = await store.get<AutoconfigProfile[]>(SETTINGS.KEYS.AUTOCONFIG_PROFILES);
    return value ?? [...SETTINGS.DEFAULTS.AUTOCONFIG_PROFILES];
  } catch (error) {
    throw new Error(`Failed to get autoconfig profiles: ${error}`);
  }
}

/** Persist the full profile list (callers broadcast the specific change) */
export async function saveAutoconfigProfiles(list: AutoconfigProfile[]): Promise<void> {
  try {
    const store = await getStore();
    await store.set(SETTINGS.KEYS.AUTOCONFIG_PROFILES, list);
    await store.save();
  } catch (error) {
    throw new Error(`Failed to save autoconfig profiles: ${error}`);
  }
}

/** Insert or update a profile by id, returning the updated list */
export async function upsertAutoconfigProfile(p: AutoconfigProfile): Promise<AutoconfigProfile[]> {
  const list = await getAutoconfigProfiles();
  const index = list.findIndex(existing => existing.id === p.id);
  if (index >= 0) {
    list[index] = p;
  } else {
    list.push(p);
  }
  await saveAutoconfigProfiles(list);
  emitProfilesChanged({ id: p.id, action: index >= 0 ? 'updated' : 'created' });
  return list;
}

/** Remove a profile by id, returning the updated list */
export async function deleteAutoconfigProfile(id: string): Promise<AutoconfigProfile[]> {
  const list = await getAutoconfigProfiles();
  const next = list.filter(existing => existing.id !== id);
  await saveAutoconfigProfiles(next);
  emitProfilesChanged({ id, action: 'deleted' });
  return next;
}

/** Get a single profile by id, or null when not found */
export async function getAutoconfigProfile(id: string): Promise<AutoconfigProfile | null> {
  const list = await getAutoconfigProfiles();
  return list.find(existing => existing.id === id) ?? null;
}
