// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { CircleCheck, Globe, MapPin } from 'lucide-react';
import { UI } from '../../config';
import { COMMON_LOCALES, getTimezones, hasCompleteUser, localeLabel, timezoneOffset } from '../../config/autoconfig';
import { wizardConfig } from '../../config/profileWizard';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { SearchPicker, type PickerOption } from '../shared/SearchPicker';
import { SegmentedChoice } from '../shared/SegmentedChoice';
import { WizardField } from './WizardFields';

const SEG_ICON = UI.ICON_SIZE.WIZARD_SEG;

export function WizardLocaleStep({ wizard }: { wizard: ProfileWizardState }) {
  const { t, i18n } = useTranslation();
  const { draft, patch } = wizard;
  const locale = draft.locale;
  const userComplete = hasCompleteUser(wizardConfig(draft));

  const timezones = useMemo<PickerOption[]>(() => {
    const now = new Date();
    return getTimezones().map((tz) => ({ value: tz, label: tz, meta: timezoneOffset(tz, now) }));
  }, []);
  const locales = useMemo<PickerOption[]>(
    () => COMMON_LOCALES.map((l) => ({ value: l, label: localeLabel(l, i18n.language), meta: l.split('.')[0] })),
    [i18n.language]
  );

  return (
    <>
      <SegmentedChoice
        value={locale.mode}
        ariaLabel={t('settings.autoconfig.wizard.locale.title')}
        options={[
          { value: 'location', label: t('settings.autoconfig.wizard.locale.location'), icon: <MapPin size={SEG_ICON} aria-hidden="true" /> },
          { value: 'choose', label: t('settings.autoconfig.wizard.locale.choose'), icon: <Globe size={SEG_ICON} aria-hidden="true" /> },
        ]}
        onChange={(mode) => patch('locale', { mode })}
      />
      {locale.mode === 'choose' ? (
        <div className="pw-two">
          <WizardField label={t('settings.autoconfig.timezone')}>
            <SearchPicker
              value={locale.timezone}
              options={timezones}
              onChange={(timezone) => patch('locale', { timezone })}
              label={t('settings.autoconfig.timezone')}
              searchPlaceholder={t('settings.autoconfig.wizard.locale.searchTimezone')}
              placeholder={t('settings.autoconfig.selectPlaceholder')}
              icon={<Globe size={15} className="ac-input__icon" aria-hidden="true" />}
            />
          </WizardField>
          <WizardField label={t('settings.autoconfig.wizard.locale.language')}>
            <SearchPicker
              value={locale.locale}
              options={locales}
              onChange={(value) => patch('locale', { locale: value })}
              label={t('settings.autoconfig.wizard.locale.language')}
              searchPlaceholder={t('settings.searchLanguage')}
              placeholder={t('settings.autoconfig.selectPlaceholder')}
            />
          </WizardField>
        </div>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.locale.locationLine')}</p>
      )}
      <div className={`pw-hint pw-hint--line${userComplete ? ' is-ok' : ''}`}>
        {userComplete && <CircleCheck size={UI.ICON_SIZE.WIZARD_HINT} aria-hidden="true" />}
        {t(userComplete ? 'settings.autoconfig.wizard.locale.userComplete' : 'settings.autoconfig.wizard.locale.userIncomplete')}
      </div>
    </>
  );
}
