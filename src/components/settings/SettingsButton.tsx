import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings } from 'lucide-react';
import { SettingsModal } from './SettingsModal';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';

/**
 * Settings button component
 *
 * Displays a settings icon in the bottom-right corner that opens the settings modal.
 * Replaces the old AppVersion component.
 */
export function SettingsButton() {
  const { t } = useTranslation();
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  const { isExiting, handleClose } = useModalExitAnimation({
    onClose: () => setIsSettingsOpen(false),
    duration: 200,
  });

  /**
   * Open settings modal
   */
  const handleOpenSettings = () => {
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
        isOpen={isSettingsOpen && !isExiting}
        onClose={handleClose}
      />
    </>
  );
}
