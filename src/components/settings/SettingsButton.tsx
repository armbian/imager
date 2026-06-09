import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings } from 'lucide-react';
import { SettingsModal, type SettingsView } from './SettingsModal';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { EVENTS } from '../../config';

interface SettingsButtonProps {
  /** 'floating' = fixed bottom-right; 'inline' = sits within a toolbar/sidebar. */
  variant?: 'floating' | 'inline';
}

export function SettingsButton({ variant = 'floating' }: SettingsButtonProps) {
  const { t } = useTranslation();
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [openCount, setOpenCount] = useState(0);
  const [initialView, setInitialView] = useState<SettingsView>('appearance');
  const [startProfileCreation, setStartProfileCreation] = useState(false);

  // Close settings when a cached image is selected for reuse
  useEffect(() => {
    const handler = () => setIsSettingsOpen(false);
    window.addEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
    return () => window.removeEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
  }, []);

  // Open settings on a requested tab, optionally starting a new profile.
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<{ view?: SettingsView; createProfile?: boolean }>).detail;
      setInitialView(detail?.view ?? 'appearance');
      setStartProfileCreation(!!detail?.createProfile);
      setOpenCount((c) => c + 1);
      setIsSettingsOpen(true);
    };
    window.addEventListener(EVENTS.OPEN_SETTINGS, handler);
    return () => window.removeEventListener(EVENTS.OPEN_SETTINGS, handler);
  }, []);

  const { isExiting, handleClose } = useModalExitAnimation({
    onClose: () => setIsSettingsOpen(false),
    duration: 200,
  });

  const handleOpenSettings = () => {
    setInitialView('appearance');
    setStartProfileCreation(false);
    setOpenCount((c) => c + 1);
    setIsSettingsOpen(true);
  };

  return (
    <>
      <button
        className={`settings-button${variant === 'inline' ? ' settings-button--inline' : ''}`}
        onClick={handleOpenSettings}
        title={t('settings.title')}
        aria-label={t('settings.title')}
      >
        <Settings size={22} strokeWidth={2} />
      </button>

      <SettingsModal
        key={openCount}
        isOpen={isSettingsOpen && !isExiting}
        onClose={handleClose}
        initialView={initialView}
        startProfileCreation={startProfileCreation}
      />
    </>
  );
}
