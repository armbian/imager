import { Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useUpdate } from '../../contexts/UpdateContext';

// Sidebar entry shown under the MOTD tip when an update is available; opens the dialog.
export function UpdateEntry() {
  const { t } = useTranslation();
  const { available, update, open } = useUpdate();

  if (!available || !update) return null;

  return (
    <div className="rail-update">
      <span className="rail-update__icon">
        <Download size={15} />
      </span>
      <span className="rail-update__text">
        {t('update.title')}
        <span>v{update.version}</span>
      </span>
      <button type="button" className="rail-update__cta" onClick={open}>
        {t('update.install')}
      </button>
    </div>
  );
}
