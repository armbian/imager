// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Armbian and Armbian Imager contributors

// Global toast system via React Context: one shared container, accessed through useToasts()

import { createContext, useContext, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import { Toast } from '../components/shared/Toast';

interface ToastItem {
  id: number;
  message: string;
  type: 'success' | 'error';
}

interface ToastContextValue {
  showSuccess: (message: string) => void;
  showError: (message: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

// Provider that manages and renders all toasts; wrap the app so children can use useToasts()
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const removeToast = useCallback((id: number) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  }, []);

  const showSuccess = useCallback((message: string) => {
    const id = Date.now();
    setToasts(prev => [...prev, { id, message, type: 'success' }]);
  }, []);

  const showError = useCallback((message: string) => {
    const id = Date.now();
    setToasts(prev => [...prev, { id, message, type: 'error' }]);
  }, []);

  return (
    <ToastContext.Provider value={{ showSuccess, showError }}>
      {children}
      <div className="toast-container">
        {toasts.map(toast => (
          <Toast
            key={toast.id}
            message={toast.message}
            type={toast.type}
            onClose={() => removeToast(toast.id)}
          />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

/** Access the toast functions (showSuccess/showError); must be inside a ToastProvider */
// eslint-disable-next-line react-refresh/only-export-components
export function useToasts(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    console.warn('useToasts must be used within a ToastProvider');
    return { showSuccess: () => {}, showError: () => {} };
  }
  return context;
}
