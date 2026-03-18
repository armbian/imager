import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings } from 'lucide-react';
import { SettingsModal } from './SettingsModal';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { EVENTS } from '../../config';

/**
 * Settings button component
 *
 * Displays a settings icon in the bottom-right corner that opens the settings modal.
 */
export function SettingsButton() {
  const { t } = useTranslation();
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [openCount, setOpenCount] = useState(0);

  // Close settings when a cached image is selected for reuse
  useEffect(() => {
    const handler = () => setIsSettingsOpen(false);
    window.addEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
    return () => window.removeEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
  }, []);

  const { isExiting, handleClose } = useModalExitAnimation({
    onClose: () => setIsSettingsOpen(false),
    duration: 200,
  });

  /** Open settings modal */
  const handleOpenSettings = () => {
    setOpenCount((c) => c + 1);
    setIsSettingsOpen(true);
  };

  return (
    <>
      <button
        className="settings-button"
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
      />
    </>
  );
}
