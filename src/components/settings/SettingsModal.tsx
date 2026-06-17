import { useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { X, Settings, Terminal, HardDrive, FileCog, Sun, Info } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { AppearanceSection } from './AppearanceSection';
import { PreferencesSection } from './PreferencesSection';
import { AutoconfigSection } from './AutoconfigSection';
import { StorageSection } from './StorageSection';
import { DeveloperSection } from './DeveloperSection';
import { AboutSection } from './AboutSection';

/** Identifier for each navigable settings section. */
export type SettingsView = 'appearance' | 'preferences' | 'profiles' | 'storage' | 'developer' | 'about';

/** Declarative description of a single navigation entry. */
interface NavItem {
  /** Section this entry activates. */
  id: SettingsView;
  /** Lucide icon rendered in the nav rail. */
  icon: LucideIcon;
  /** i18n key for the nav label. */
  labelKey: string;
}

/** Nav entries in display order. About uses `settings.appInfo` as label while AboutSection keeps the
 * hard-coded "Armbian Imager" title — mismatch is intentional and preserved. */
const NAV_ITEMS: readonly NavItem[] = [
  { id: 'appearance', icon: Sun, labelKey: 'settings.appearance' },
  { id: 'preferences', icon: Settings, labelKey: 'settings.preferences' },
  { id: 'profiles', icon: FileCog, labelKey: 'settings.autoconfig.tab' },
  { id: 'storage', icon: HardDrive, labelKey: 'settings.storage' },
  { id: 'developer', icon: Terminal, labelKey: 'settings.developer' },
  { id: 'about', icon: Info, labelKey: 'settings.appInfo' },
];

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialView?: SettingsView;
  startProfileCreation?: boolean;
}

/** Frosted-glass Settings island: nav rail + active section, mirroring `.split-nav`/`.split-main`. Portaled to `document.body` so the overlay
 * escapes transformed/stacked ancestors and covers the window. `isOpen` controls visibility; `onClose` fires on overlay or close-button activation. */
export function SettingsModal({ isOpen, onClose, initialView = 'appearance', startProfileCreation = false }: SettingsModalProps) {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingsView>(initialView);

  if (!isOpen) return null;

  /** Renders the currently selected section component. */
  const renderSection = () => {
    switch (activeSection) {
      case 'appearance':
        return <AppearanceSection />;
      case 'preferences':
        return <PreferencesSection />;
      case 'profiles':
        // Launched from the flash flow: return there once the profile is saved.
        return <AutoconfigSection autoCreate={startProfileCreation} onSaved={startProfileCreation ? onClose : undefined} />;
      case 'storage':
        return <StorageSection />;
      case 'developer':
        return <DeveloperSection />;
      case 'about':
        return <AboutSection />;
    }
  };

  // Portal to <body> so the overlay escapes any transformed/stacked ancestor
  // (e.g. the animated .split) and reliably covers the whole window, header included.
  return createPortal(
    <div className="modal-overlay modal-entering" onClick={onClose}>
      <div
        className="settings-shell"
        role="dialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="settings-shell__header">
          <h2 className="settings-shell__title">{t('settings.title')}</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={20} />
          </button>
        </header>

        <div className="settings-shell__body">
          <nav className="settings-nav">
            {NAV_ITEMS.map(({ id, icon: Icon, labelKey }) => {
              const isActive = activeSection === id;
              return (
                <button
                  key={id}
                  className={`settings-nav__item ${isActive ? 'settings-nav__item--active' : ''}`}
                  aria-current={isActive ? 'page' : undefined}
                  onClick={() => setActiveSection(id)}
                >
                  <Icon size={20} className="settings-nav__icon" />
                  <span>{t(labelKey)}</span>
                </button>
              );
            })}
          </nav>

          <div className="settings-shell__content">{renderSection()}</div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
