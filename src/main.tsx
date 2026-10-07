// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { initI18n } from './i18n';
import { ThemeProvider } from './contexts/ThemeContext';
import { MotionProvider } from './contexts/MotionContext';
import { PLATFORM, PLATFORM_CLASS } from './config';
import { uiPlatform } from './utils';

// Tag the platform so the layout can reserve space for the overlay traffic lights (macOS)
if (uiPlatform() === PLATFORM.MACOS) {
  document.documentElement.classList.add(PLATFORM_CLASS.MACOS);
}

// Disable context menu in production
if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => e.preventDefault());
}

// Initialize i18n before rendering
initI18n().then(() => {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <ThemeProvider>
        <MotionProvider>
          <App />
        </MotionProvider>
      </ThemeProvider>
    </React.StrictMode>
  );
});
