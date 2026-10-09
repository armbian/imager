// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, type Ref } from 'react';
import { useTranslation } from 'react-i18next';
import { SETTINGS_VIEW } from '../../config';
import { NAV_ICONS } from '../../config/heroArt';
import { useSettingsSummary } from '../../hooks/useSettingsSummary';
import type { SettingsView } from '../../types';

const NAV_ORDER: readonly SettingsView[] = [
  SETTINGS_VIEW.GENERAL,
  SETTINGS_VIEW.WRITING,
  SETTINGS_VIEW.PROFILES,
  SETTINGS_VIEW.DOWNLOADS,
  SETTINGS_VIEW.ABOUT,
];

const NAV_LABEL_KEY: Record<SettingsView, string> = {
  general: 'settings.nav.general',
  writing: 'settings.nav.writing',
  profiles: 'settings.nav.profiles',
  downloads: 'settings.nav.downloads',
  about: 'settings.nav.about',
};

interface SettingsNavProps {
  view: SettingsView;
  onSelect: (view: SettingsView) => void;
  activeRef?: Ref<HTMLButtonElement>;
}

export function SettingsNav({ view, onSelect, activeRef }: SettingsNavProps) {
  const { t } = useTranslation();
  const summary = useSettingsSummary();
  const titleId = useId();

  return (
    <nav className="settings-page__island settings-page__nav" aria-labelledby={titleId}>
      <h2 id={titleId} className="settings-side__title">
        {t('settings.title')}
      </h2>
      <div className="settings-side__list">
        {NAV_ORDER.map((id) => {
          const active = id === view;
          return (
            <button
              key={id}
              ref={active ? activeRef : undefined}
              type="button"
              className={`settings-side__item${active ? ' is-active' : ''}`}
              aria-current={active ? 'page' : undefined}
              onClick={() => {
                if (!active) onSelect(id);
              }}
            >
              <img className="settings-side__icon" src={NAV_ICONS[id]} alt="" aria-hidden="true" />
              <span className="settings-side__text">
                <span className="settings-side__label">{t(NAV_LABEL_KEY[id])}</span>
                <span className="settings-side__sub">{summary[id]}</span>
              </span>
            </button>
          );
        })}
      </div>
    </nav>
  );
}
