// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Globe, MapPin } from 'lucide-react';
import { COMMON_LOCALES, getTimezones, localeLabel, timezoneOffset } from '../../../config/autoconfig';
import { localeCountry, type RegionMode } from '../../../config/profileEditorModel';
import { SegmentedControl, SelectMenu, SettingsRow, type SelectOption } from '../controls';
import { CountryFlag, DefaultsAction, EditorSection, FieldHint } from './EditorParts';
import type { EditorGroupProps } from './types';

export function RegionGroup({ model, status, patch, keepDefaults }: EditorGroupProps) {
  const { t, i18n } = useTranslation();
  const r = model.region;
  const choose = r.apply && r.mode === 'choose';
  const placeholder = t('settings.autoconfig.selectPlaceholder');

  const locales = useMemo<SelectOption<string>[]>(
    () => [
      { value: '', label: placeholder },
      ...COMMON_LOCALES.map((l) => {
        const country = localeCountry(l);
        return {
          value: l,
          label: localeLabel(l, i18n.language),
          icon: country ? <CountryFlag code={country} /> : <Globe aria-hidden="true" />,
          badge: l.split('.')[0],
        };
      }),
    ],
    [placeholder, i18n.language]
  );
  const timezones = useMemo<SelectOption<string>[]>(() => {
    const now = new Date();
    return [
      { value: '', label: placeholder },
      ...getTimezones().map((tz) => ({ value: tz, label: tz, badge: timezoneOffset(tz, now) || undefined })),
    ];
  }, [placeholder]);

  return (
    <EditorSection
      id="region"
      eyebrow={t('settings.autoconfig.wizard.steps.locale')}
      description={t('settings.autoconfig.wizard.locale.sub')}
      attention={status === 'attn'}
      action={r.apply ? <DefaultsAction onClick={(byPointer) => keepDefaults('region', byPointer)} /> : undefined}
    >
      <SettingsRow
        label={t('settings.autoconfig.wizard.steps.locale')}
        tall={!choose}
        description={
          choose ? undefined : (
            <FieldHint>{t(r.apply ? 'settings.autoconfig.editor.locationHint' : 'settings.autoconfig.armbianDefaults')}</FieldHint>
          )
        }
        control={
          <SegmentedControl<RegionMode>
            value={r.apply ? r.mode : null}
            ariaLabel={t('settings.autoconfig.wizard.steps.locale')}
            options={[
              { value: 'location', label: t('settings.autoconfig.wizard.locale.location'), icon: <MapPin aria-hidden="true" /> },
              { value: 'choose', label: t('settings.autoconfig.wizard.locale.choose'), icon: <Globe aria-hidden="true" /> },
            ]}
            onChange={(mode) => patch('region', { mode })}
          />
        }
      />
      {choose && (
        <>
          <SettingsRow
            label={t('settings.autoconfig.wizard.locale.language')}
            control={
              <SelectMenu
                value={r.locale}
                options={locales}
                onChange={(locale) => patch('region', { locale })}
                ariaLabel={t('settings.autoconfig.wizard.locale.language')}
                searchable
                searchPlaceholder={t('settings.searchLanguage')}
              />
            }
          />
          <SettingsRow
            label={t('settings.autoconfig.timezone')}
            control={
              <SelectMenu
                value={r.timezone}
                options={timezones}
                onChange={(timezone) => patch('region', { timezone })}
                ariaLabel={t('settings.autoconfig.timezone')}
                searchable
                searchPlaceholder={t('settings.autoconfig.wizard.locale.searchTimezone')}
              />
            }
          />
        </>
      )}
    </EditorSection>
  );
}
