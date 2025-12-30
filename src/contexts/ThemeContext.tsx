import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { getTheme, setTheme as saveTheme } from '../hooks/useSettings';

/**
 * Theme type
 */
export type Theme = 'light' | 'dark' | 'auto';

/**
 * Theme context interface
 */
interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

/**
 * Theme context
 */
const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

/**
 * Theme provider props
 */
interface ThemeProviderProps {
  children: ReactNode;
}

/**
 * Theme provider component
 *
 * Manages theme state and applies theme classes to the document element.
 * Persists theme preference via Tauri backend commands.
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>('auto');
  const [isInitialized, setIsInitialized] = useState(false);

  /**
   * Apply theme to document element
   */
  const applyTheme = (selectedTheme: Theme) => {
    const root = document.documentElement;

    if (selectedTheme === 'light') {
      root.classList.add('theme-light');
      root.classList.remove('theme-dark');
    } else if (selectedTheme === 'dark') {
      root.classList.add('theme-dark');
      root.classList.remove('theme-light');
    } else {
      // auto - remove both classes, let CSS media query handle it
      root.classList.remove('theme-light', 'theme-dark');
    }
  };

  /**
   * Load theme from storage on mount
   */
  useEffect(() => {
    const loadTheme = async () => {
      try {
        const savedTheme = await getTheme();
        setThemeState(savedTheme as Theme);
        applyTheme(savedTheme as Theme);
      } catch (error) {
        // If no saved theme, default to auto
        console.warn('Failed to load theme from storage, using auto:', error);
        setThemeState('auto');
        applyTheme('auto');
      } finally {
        setIsInitialized(true);
      }
    };

    loadTheme();
  }, []);

  /**
   * Set theme and persist to storage
   */
  const setTheme = async (newTheme: Theme) => {
    setThemeState(newTheme);
    applyTheme(newTheme);

    // Persist to storage
    try {
      await saveTheme(newTheme);
    } catch (error) {
      console.error('Failed to save theme to storage:', error);
    }
  };

  const value = {
    theme,
    setTheme,
  };

  // Don't render children until theme is loaded to prevent flash
  if (!isInitialized) {
    return null;
  }

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

/**
 * Hook to access theme context
 *
 * @throws Error if used outside ThemeProvider
 */
// eslint-disable-next-line react-refresh/only-export-components -- This is a hook, not a component
export function useTheme(): ThemeContextType {
  const context = useContext(ThemeContext);

  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }

  return context;
}
