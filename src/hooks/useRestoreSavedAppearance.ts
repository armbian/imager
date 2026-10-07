// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useTheme, type Theme } from '../contexts/ThemeContext';
import { useMotion, type MotionMode } from '../contexts/MotionContext';
import { getLanguage, getReducedMotion, getTheme } from './useSettings';
import { changeLanguage as changeSavedLanguage } from '../i18n';
import { AUTO_LANGUAGE_CODE } from '../config/i18n';

export function useRestoreSavedAppearance(): () => Promise<void> {
  const { i18n } = useTranslation();
  const { setTheme } = useTheme();
  const { setMotion } = useMotion();

  return useCallback(async () => {
    const [savedTheme, savedMotion, savedLanguage] = await Promise.all([
      getTheme(),
      getReducedMotion(),
      getLanguage(),
    ]);
    setTheme(savedTheme as Theme, { persist: false });
    setMotion(savedMotion as MotionMode, { persist: false });
    if (savedLanguage) await i18n.changeLanguage(savedLanguage);
    else await changeSavedLanguage(AUTO_LANGUAGE_CODE);
  }, [i18n, setTheme, setMotion]);
}
