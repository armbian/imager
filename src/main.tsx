import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { initI18n } from './i18n';
import { ThemeProvider } from './contexts/ThemeContext';

// Tag the platform so the layout can reserve space for the overlay traffic lights (macOS)
if (navigator.userAgent.includes('Mac')) {
  document.documentElement.classList.add('is-macos');
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
        <App />
      </ThemeProvider>
    </React.StrictMode>
  );
});
