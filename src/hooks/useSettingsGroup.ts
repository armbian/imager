/**
 * Batch settings loader hook
 *
 * Loads multiple settings on mount with a single useEffect.
 * Reduces boilerplate and consolidates error handling.
 */

import { useEffect, useState } from 'react';

type SettingsLoader<T> = () => Promise<T>;

export function useSettingsGroup<T extends Record<string, unknown>>(
  config: Record<keyof T, SettingsLoader<unknown>>
): Partial<T> {
  const [values, setValues] = useState<Partial<T>>({});

  useEffect(() => {
    const loadAllSettings = async () => {
      const entries = Object.entries(config) as Array<[string, SettingsLoader<unknown>]>;

      // Load all settings in parallel
      const results = await Promise.all(
        entries.map(async ([key, loader]) => {
          try {
            const value = await loader();
            return [key, value] as const;
          } catch (error) {
            console.error(`Failed to load setting "${key}":`, error);
            return [key, undefined] as const;
          }
        })
      );

      // Update state with all loaded values
      const newValues = results.reduce((acc, [key, value]) => {
        if (value !== undefined) {
          acc[key as keyof T] = value as T[keyof T];
        }
        return acc;
      }, {} as Partial<T>);

      setValues(newValues);
    };

    loadAllSettings();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- Only run on mount, config is stable
  }, []);

  return values;
}
