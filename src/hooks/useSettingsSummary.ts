// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import { CACHE, EVENTS, SETTINGS, SETTINGS_VIEW, UI } from '../config/constants';
import { AUTO_LANGUAGE_CODE, SUPPORTED_LANGUAGES } from '../config/i18n';
import { getSystemLanguage } from '../i18n';
import { useTheme } from '../contexts/ThemeContext';
import { formatBytes } from '../utils';
import type { SettingsView } from '../types';
import { getAutoconfigProfiles, getCacheMaxSize, getLanguage, getSkipVerify } from './useSettings';
import { getCacheBreakdown, logWarn } from './useTauri';

interface SummarySnapshot {
  language: string | null;
  systemLanguage: string;
  skipVerify: boolean;
  profileCount: number;
  cacheUsed: number;
  cacheLimit: number;
  version: string;
}

const THEME_LABEL_KEY = {
  [SETTINGS.THEME_MODES.LIGHT]: 'settings.themeLight',
  [SETTINGS.THEME_MODES.DARK]: 'settings.themeDark',
  [SETTINGS.THEME_MODES.AUTO]: 'settings.themeAuto',
} as const;

async function readSnapshot(): Promise<SummarySnapshot> {
  // One failing source must not blank the whole nav
  const [language, systemLanguage, skipVerify, profiles, breakdown, cacheLimit, version] = await Promise.all([
    getLanguage().catch(() => null),
    getSystemLanguage(),
    getSkipVerify().catch(() => SETTINGS.DEFAULTS.SKIP_VERIFY),
    getAutoconfigProfiles().catch(() => []),
    getCacheBreakdown().catch(() => CACHE.EMPTY_BREAKDOWN),
    getCacheMaxSize().catch(() => CACHE.DEFAULT_SIZE),
    getVersion().catch(() => ''),
  ]);
  return {
    language,
    systemLanguage,
    skipVerify,
    profileCount: profiles.length,
    cacheUsed: breakdown.total,
    cacheLimit,
    version,
  };
}

/** One-line state of every settings view for the nav, kept fresh as settings and profiles change. */
export function useSettingsSummary(): Record<SettingsView, string> {
  const { t, i18n } = useTranslation();
  const { theme } = useTheme();
  const [snapshot, setSnapshot] = useState<SummarySnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    const refresh = () => {
      readSnapshot()
        .then((next) => {
          if (!cancelled) setSnapshot(next);
        })
        .catch((error) => {
          logWarn('settings', `Failed to read the settings summary: ${error}`);
        });
    };

    refresh();
    window.addEventListener(EVENTS.SETTINGS_CHANGED, refresh);
    window.addEventListener(EVENTS.PROFILES_CHANGED, refresh);
    return () => {
      cancelled = true;
      window.removeEventListener(EVENTS.SETTINGS_CHANGED, refresh);
      window.removeEventListener(EVENTS.PROFILES_CHANGED, refresh);
    };
  }, [i18n.language]);

  const languageCode = snapshot ? (snapshot.language && snapshot.language !== AUTO_LANGUAGE_CODE ? snapshot.language : snapshot.systemLanguage) : null;
  const languageName = languageCode
    ? (SUPPORTED_LANGUAGES.find((l) => l.code === languageCode)?.name ?? languageCode)
    : t('settings.languageAuto');

  return {
    [SETTINGS_VIEW.GENERAL]: [t(THEME_LABEL_KEY[theme]), languageName].join(UI.SUMMARY_SEPARATOR),
    [SETTINGS_VIEW.WRITING]: snapshot
      ? t(snapshot.skipVerify ? 'settings.nav.verifyOff' : 'settings.nav.verifyOn')
      : '',
    [SETTINGS_VIEW.PROFILES]: snapshot ? t('settings.nav.profileCount', { count: snapshot.profileCount }) : '',
    [SETTINGS_VIEW.DOWNLOADS]: snapshot
      ? t('settings.nav.cacheUsed', {
          used: formatBytes(snapshot.cacheUsed),
          limit: formatBytes(snapshot.cacheLimit),
        })
      : '',
    [SETTINGS_VIEW.ABOUT]: snapshot?.version ? t('settings.nav.versionLine', { version: snapshot.version }) : '',
  };
}
