// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Cable, Globe, Network, Wifi } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, WIFI_COUNTRY_CODES } from '../../config/autoconfig';
import { countryName, editorIpMissing, hasWifi, wifiIssues, type IpMode, type NetMode } from '../../config/profileEditorModel';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import type { StaticIpErrors } from '../../utils';
import { SegmentedControl, SelectMenu, type SelectOption } from '../settings/controls';
import { CountryFlag, FieldHint } from '../settings/editor/EditorParts';
import { WifiKeyHint } from '../settings/editor/EditorParts';
import { PasswordInput } from '../shared/PasswordInput';
import { WizardField, WizardMore, WizardTextField } from './WizardFields';

const IP_FIELDS: { field: keyof StaticIpErrors; key: 'address' | 'mask' | 'gateway' | 'dns'; label: string; placeholder: string }[] = [
  { field: 'ip', key: 'address', label: 'address', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_IP },
  { field: 'mask', key: 'mask', label: 'netmask', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_MASK },
  { field: 'gateway', key: 'gateway', label: 'gateway', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_GATEWAY },
  { field: 'dns', key: 'dns', label: 'dns', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_DNS },
];

export function WizardNetworkStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t, i18n } = useTranslation();
  const { model, more, attempted, stepError, ipErrors, patch, toggleMore } = wizard;
  const n = model.network;
  const wifi = hasWifi(n.mode);
  const wifiProblems = wifiIssues(n);
  // An empty name complains once Next was pressed; characters it cannot write complain as you type
  const ssidProblem = wifiProblems.ssid && (attempted || !!n.ssid.trim()) ? wifiProblems.ssid : undefined;
  const missing: ReturnType<typeof editorIpMissing> = attempted && !!stepError ? editorIpMissing(n) : {};
  const ipMessages = [
    missing.ip && t(`settings.autoconfig.${missing.ip}`),
    missing.mask && t(`settings.autoconfig.${missing.mask}`),
    ...Object.values(ipErrors).map((key) => t(`settings.autoconfig.${key}`)),
  ].filter((message): message is string => !!message);

  const countries = useMemo<SelectOption<string>[]>(
    () => [
      { value: '', label: t('settings.autoconfig.selectPlaceholder'), icon: <Globe aria-hidden="true" /> },
      ...WIFI_COUNTRY_CODES.map((c) => ({
        value: c.code,
        label: countryName(c.code, c.name, i18n.language),
        icon: <CountryFlag code={c.code} />,
        badge: c.code,
      })),
    ],
    [t, i18n.language]
  );

  return (
    <>
      <SegmentedControl<NetMode>
        value={n.apply ? n.mode : null}
        ariaLabel={t('settings.autoconfig.wizard.network.title')}
        options={[
          { value: 'cable', label: t('settings.autoconfig.wizard.network.cable'), icon: <Cable aria-hidden="true" /> },
          { value: 'wifi', label: t('settings.autoconfig.wizard.network.wifi'), icon: <Wifi aria-hidden="true" /> },
          { value: 'both', label: t('settings.autoconfig.wizard.network.both'), icon: <Network aria-hidden="true" /> },
        ]}
        onChange={(mode) => patch('network', { mode, apply: true })}
      />
      {!n.apply ? (
        <p className="pw-line">{t('settings.autoconfig.wizard.pending.network')}</p>
      ) : wifi ? (
        <>
          <div className="pw-two">
            <WizardField
              label={t('settings.autoconfig.wizard.network.ssid')}
              htmlFor={`${uid}-ssid`}
              hint={ssidProblem ? <FieldHint tone="warn">{t(ssidProblem)}</FieldHint> : undefined}
            >
              <WizardTextField
                id={`${uid}-ssid`}
                value={n.ssid}
                icon={<Wifi size={15} className="pe-input__icon" aria-hidden="true" />}
                placeholder={AUTOCONFIG_PLACEHOLDERS.WIFI_SSID}
                invalid={!!ssidProblem}
                onChange={(ssid) => patch('network', { ssid })}
              />
            </WizardField>
            <WizardField label={t('settings.autoconfig.wizard.password')} htmlFor={`${uid}-key`} hint={<WifiKeyHint wifiKey={n.key} issue={wifiProblems.key} />}>
              <PasswordInput id={`${uid}-key`} value={n.key} invalid={!!wifiProblems.key} onChange={(key) => patch('network', { key })} />
            </WizardField>
          </div>
          <FieldHint>{t('settings.autoconfig.editor.wifiNoAdapterHint')}</FieldHint>
        </>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.network.cableLine')}</p>
      )}
      {n.apply && (
        <WizardMore
          label={t(wifi ? 'settings.autoconfig.wizard.network.moreWifi' : 'settings.autoconfig.wizard.network.moreCable')}
          open={!!more.network}
          onToggle={() => toggleMore('network')}
        />
      )}
      {n.apply && more.network && (
        <div className="pw-two">
          {wifi && (
            <WizardField label={t('settings.autoconfig.wizard.network.country')}>
              <SelectMenu
                value={n.country}
                options={countries}
                onChange={(country) => patch('network', { country })}
                ariaLabel={t('settings.autoconfig.wizard.network.country')}
                searchable
                searchPlaceholder={t('settings.autoconfig.wizard.network.searchCountry')}
              />
            </WizardField>
          )}
          <WizardField
            label={t(n.mode === 'both' ? 'settings.autoconfig.editor.ipLabelWifi' : 'settings.autoconfig.wizard.network.ip')}
            hint={n.mode === 'both' ? <FieldHint>{t('settings.autoconfig.editor.bothCableAuto')}</FieldHint> : undefined}
          >
            <SegmentedControl<IpMode>
              value={n.ip}
              ariaLabel={t('settings.autoconfig.wizard.network.ip')}
              options={[
                { value: 'auto', label: t('settings.autoconfig.wizard.network.ipAuto') },
                { value: 'fixed', label: t('settings.autoconfig.wizard.network.ipFixed') },
              ]}
              onChange={(ip) => patch('network', { ip })}
            />
          </WizardField>
        </div>
      )}
      {n.apply && more.network && n.ip === 'fixed' && (
        <div className="pw-four">
          {IP_FIELDS.map(({ field, key, label, placeholder }) => (
            <WizardField key={key} label={t(`settings.autoconfig.wizard.network.${label}`)} htmlFor={`${uid}-ip-${key}`}>
              <WizardTextField
                id={`${uid}-ip-${key}`}
                mono
                value={n[key]}
                placeholder={placeholder}
                invalid={!!ipErrors[field] || (field === 'ip' && !!missing.ip) || (field === 'mask' && !!missing.mask)}
                onChange={(value) => patch('network', { [key]: value })}
              />
            </WizardField>
          ))}
          {ipMessages.length > 0 && (
            // Four columns are too narrow for the messages, so they run under the whole row.
            <div className="pw-four__msgs" role="alert">
              {ipMessages.map((message) => (
                <FieldHint key={message} tone="warn">{message}</FieldHint>
              ))}
            </div>
          )}
        </div>
      )}
    </>
  );
}
