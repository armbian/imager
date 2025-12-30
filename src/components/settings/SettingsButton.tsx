import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings } from 'lucide-react';
import { SettingsModal } from './SettingsModal';

/**
 * Settings button component
 *
 * Displays a settings icon in the bottom-right corner that opens the settings modal.
 * Replaces the old AppVersion component.
 */
export function SettingsButton() {
  const { t } = useTranslation();
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const isExitingRef = useState(false);

  /**
   * Open settings modal
   */
  const handleOpenSettings = () => {
    if (isExitingRef[0]) return;
    setIsSettingsOpen(true);
  };

  /**
   * Close settings modal with animation
   */
  const handleCloseSettings = () => {
    if (isExiting) return;
    setIsExiting(true);
    setTimeout(() => {
      setIsSettingsOpen(false);
      setIsExiting(false);
    }, 200);
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
        onClose={handleCloseSettings}
      />
    </>
  );
}
