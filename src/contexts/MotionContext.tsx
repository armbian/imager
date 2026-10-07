// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { getReducedMotion, setReducedMotion as saveMotion } from '../hooks/useSettings';
import { SETTINGS } from '../config';
import type { ApplyOptions } from './ThemeContext';

export type MotionMode = (typeof SETTINGS.MOTION_MODES)[keyof typeof SETTINGS.MOTION_MODES];

interface MotionContextType {
  motion: MotionMode;
  setMotion: (mode: MotionMode, options?: ApplyOptions) => void;
}

const MotionContext = createContext<MotionContextType | undefined>(undefined);

// Stamps the choice on the document element so CSS can override the OS media query in
// both directions: 'true' forces animations off, 'false' keeps them on despite the OS.
function applyMotion(mode: MotionMode) {
  const root = document.documentElement;

  if (mode === SETTINGS.MOTION_MODES.REDUCE) {
    root.setAttribute('data-reduced-motion', 'true');
  } else if (mode === SETTINGS.MOTION_MODES.FULL) {
    root.setAttribute('data-reduced-motion', 'false');
  } else {
    root.removeAttribute('data-reduced-motion');
  }
}

/** Manages the motion preference, applies it to the document element, and persists it. */
export function MotionProvider({ children }: { children: ReactNode }) {
  const [motion, setMotionState] = useState<MotionMode>(SETTINGS.MOTION_MODES.AUTO);

  useEffect(() => {
    getReducedMotion()
      .then((saved) => {
        setMotionState(saved as MotionMode);
        applyMotion(saved as MotionMode);
      })
      .catch((error) => {
        console.warn('Failed to load motion preference, following the system:', error);
      });
  }, []);

  const setMotion = async (mode: MotionMode, { persist = true }: ApplyOptions = {}) => {
    setMotionState(mode);
    applyMotion(mode);
    if (!persist) return;

    try {
      await saveMotion(mode);
    } catch (error) {
      console.error('Failed to save motion preference:', error);
    }
  };

  return <MotionContext.Provider value={{ motion, setMotion }}>{children}</MotionContext.Provider>;
}

/** Access the motion context (throws if used outside MotionProvider) */
// eslint-disable-next-line react-refresh/only-export-components -- This is a hook, not a component
export function useMotion(): MotionContextType {
  const context = useContext(MotionContext);

  if (!context) {
    throw new Error('useMotion must be used within a MotionProvider');
  }

  return context;
}
