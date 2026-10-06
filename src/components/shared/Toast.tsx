// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

// Auto-dismissing success/error notification toast

import { useState, useEffect } from 'react';
import { TIMING, UI } from '../../config';

export interface ToastProps {
  message: string;
  type: 'success' | 'error';
  duration?: number;
  onClose: () => void;
}

export function Toast({ message, type, duration = TIMING.TOAST_DURATION, onClose }: ToastProps) {
  const [isVisible, setIsVisible] = useState(true);

  useEffect(() => {
    const outerTimer = setTimeout(() => {
      setIsVisible(false);
      const innerTimer = setTimeout(onClose, UI.TOAST_EXIT_MS);
      return () => clearTimeout(innerTimer);
    }, duration);

    return () => clearTimeout(outerTimer);
  }, [duration, onClose]);

  const icons = {
    success: '✓',
    error: '✕'
  };

  return (
    <div
      role="status"
      aria-live="polite"
      className={`toast toast-${type} ${isVisible ? 'toast-enter' : 'toast-exit'}`}
    >
      <span className="toast-icon">{icons[type]}</span>
      <span className="toast-message">{message}</span>
      <button className="toast-close" onClick={() => setIsVisible(false)} aria-label="Close">
        ✕
      </button>
    </div>
  );
}
