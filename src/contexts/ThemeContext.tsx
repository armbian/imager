// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { getTheme, setTheme as saveTheme } from '../hooks/useSettings';
import { SETTINGS } from '../config';
import { useMediaQuery } from '../hooks/useMediaQuery';

export type Theme = (typeof SETTINGS.THEME_MODES)[keyof typeof SETTINGS.THEME_MODES];
export type ResolvedTheme = typeof SETTINGS.THEME_MODES.LIGHT | typeof SETTINGS.THEME_MODES.DARK;

const DARK_SCHEME_QUERY = '(prefers-color-scheme: dark)';

export interface ApplyOptions {
  persist?: boolean;
}

interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme, options?: ApplyOptions) => void;
  /** What is on screen: the chosen theme, or the OS scheme in auto */
  resolvedTheme: ResolvedTheme;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

interface ThemeProviderProps {
  children: ReactNode;
}

// Manages theme state, applies it to the document element, and persists it
export function ThemeProvider({ children }: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(SETTINGS.THEME_MODES.AUTO);
  const [isInitialized, setIsInitialized] = useState(false);
  const osDark = useMediaQuery(DARK_SCHEME_QUERY);
  const resolvedTheme: ResolvedTheme =
    theme === SETTINGS.THEME_MODES.AUTO
      ? osDark
        ? SETTINGS.THEME_MODES.DARK
        : SETTINGS.THEME_MODES.LIGHT
      : theme;

  const applyTheme = (selectedTheme: Theme) => {
    const root = document.documentElement;

    if (selectedTheme === SETTINGS.THEME_MODES.LIGHT) {
      root.classList.add('theme-light');
      root.classList.remove('theme-dark');
    } else if (selectedTheme === SETTINGS.THEME_MODES.DARK) {
      root.classList.add('theme-dark');
      root.classList.remove('theme-light');
    } else {
      root.classList.remove('theme-light', 'theme-dark');
    }
  };

  useEffect(() => {
    const loadTheme = async () => {
      try {
        const savedTheme = await getTheme();
        setThemeState(savedTheme as Theme);
        applyTheme(savedTheme as Theme);
      } catch (error) {
        console.warn('Failed to load theme from storage, using auto:', error);
        setThemeState(SETTINGS.THEME_MODES.AUTO);
        applyTheme(SETTINGS.THEME_MODES.AUTO);
      } finally {
        setIsInitialized(true);
      }
    };

    loadTheme();
  }, []);

  const setTheme = async (newTheme: Theme, { persist = true }: ApplyOptions = {}) => {
    setThemeState(newTheme);
    applyTheme(newTheme);
    if (!persist) return;

    try {
      await saveTheme(newTheme);
    } catch (error) {
      console.error('Failed to save theme to storage:', error);
    }
  };

  const value = {
    theme,
    setTheme,
    resolvedTheme,
  };

  // Don't render children until theme is loaded to prevent flash
  if (!isInitialized) {
    return null;
  }

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

/** Access the theme context (throws if used outside ThemeProvider) */
// eslint-disable-next-line react-refresh/only-export-components -- This is a hook, not a component
export function useTheme(): ThemeContextType {
  const context = useContext(ThemeContext);

  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }

  return context;
}
