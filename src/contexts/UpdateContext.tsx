import { createContext, useContext, useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { logInfo } from '../hooks/useTauri';
import { getShowUpdaterModal } from '../hooks/useSettings';

interface UpdateContextType {
  /** Pending update, or null when none is available. */
  update: Update | null;
  /** True when an update is available to install. */
  available: boolean;
  /** Whether the update dialog is open. */
  isOpen: boolean;
  open: () => void;
  close: () => void;
}

const UpdateContext = createContext<UpdateContextType | undefined>(undefined);

// Checks for an update once on launch and shares it with the sidebar entry and the
// dialog. The dialog never auto-opens; the user opens it from the sidebar entry.
export function UpdateProvider({ children }: { children: ReactNode }) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const hasCheckedRef = useRef(false);

  useEffect(() => {
    if (hasCheckedRef.current) return;
    hasCheckedRef.current = true;

    const run = async () => {
      // Honour the "notify about updates" setting; skip the check when disabled.
      const notify = await getShowUpdaterModal();
      if (!notify) {
        logInfo('updater', 'Update notifications disabled in settings, skipping check');
        return;
      }
      try {
        const result = await check();
        if (result) {
          setUpdate(result);
          logInfo('updater', `Update available: ${result.currentVersion} -> ${result.version}`);
        } else {
          logInfo('updater', 'No updates available');
        }
      } catch (err) {
        // Non-critical: the user can keep using the current version.
        console.error('Failed to check for updates:', err);
      }
    };

    run();
  }, []);

  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);

  return (
    <UpdateContext.Provider value={{ update, available: update !== null, isOpen, open, close }}>
      {children}
    </UpdateContext.Provider>
  );
}

/** Access the update context (throws if used outside UpdateProvider) */
// eslint-disable-next-line react-refresh/only-export-components -- This is a hook, not a component
export function useUpdate(): UpdateContextType {
  const context = useContext(UpdateContext);
  if (!context) {
    throw new Error('useUpdate must be used within an UpdateProvider');
  }
  return context;
}
