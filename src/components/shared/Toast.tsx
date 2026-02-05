/**
 * Toast notification component
 *
 * Displays temporary notification messages that auto-dismiss after a delay.
 * Supports success and error variants.
 */

import { useState, useEffect } from 'react';

export interface ToastProps {
  message: string;
  type: 'success' | 'error';
  duration?: number;
  onClose: () => void;
}

export function Toast({ message, type, duration = 3000, onClose }: ToastProps) {
  const [isVisible, setIsVisible] = useState(true);

  useEffect(() => {
    const outerTimer = setTimeout(() => {
      setIsVisible(false);
      const innerTimer = setTimeout(onClose, 300);
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
