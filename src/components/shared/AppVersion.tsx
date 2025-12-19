import { useState, useEffect } from 'react';
import { getVersion } from '@tauri-apps/api/app';

export function AppVersion() {
  const [version, setVersion] = useState<string>('');

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  if (!version) return null;

  return <span className="app-version">v{version}</span>;
}
