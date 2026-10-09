// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Cable, Globe, Network, Wifi } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, WIFI_COUNTRY_CODES, isWifiKeyLength } from '../../config/autoconfig';
import { countryName, hasWifi, type IpMode, type NetMode } from '../../config/profileEditorModel';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import type { StaticIpErrors } from '../../utils';
import type { TFn } from '../../utils/errorUtils';
import { SegmentedControl, SelectMenu, type SelectOption } from '../settings/controls';
import { CountryFlag, FieldHint } from '../settings/editor/EditorParts';
import { PasswordInput } from '../shared/PasswordInput';
import { WizardField, WizardMore, WizardTextField } from './WizardFields';

const IP_FIELDS: { field: keyof StaticIpErrors; key: 'address' | 'mask' | 'gateway' | 'dns'; label: string; placeholder: string }[] = [
  { field: 'ip', key: 'address', label: 'address', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_IP },
  { field: 'mask', key: 'mask', label: 'netmask', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_MASK },
  { field: 'gateway', key: 'gateway', label: 'gateway', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_GATEWAY },
  { field: 'dns', key: 'dns', label: 'dns', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_DNS },
];

function wifiKeyHint(key: string, t: TFn) {
  if (!key) return <FieldHint>{t('settings.autoconfig.editor.wifiKeyEmpty')}</FieldHint>;
  return isWifiKeyLength(key) ? (
    <FieldHint tone="ok">{t('settings.autoconfig.wifiKeyFits')}</FieldHint>
  ) : (
    <FieldHint tone="warn">{t('settings.autoconfig.wifiKeyLength')}</FieldHint>
  );
}

export function WizardNetworkStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t, i18n } = useTranslation();
  const { model, more, attempted, stepError, ipErrors, patch, toggleMore } = wizard;
  const n = model.network;
  const wifi = hasWifi(n.mode);
  const ssidMissing = attempted && stepError === 'settings.autoconfig.wizard.ssidRequired';
  const addressMissing = attempted && stepError === 'settings.autoconfig.wizard.addressRequired';
  const ipMessages = [
    addressMissing && t('settings.autoconfig.wizard.addressRequired'),
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
        <div className="pw-two">
          <WizardField
            label={t('settings.autoconfig.wizard.network.ssid')}
            htmlFor={`${uid}-ssid`}
            hint={ssidMissing ? <FieldHint tone="warn">{t('settings.autoconfig.wizard.ssidRequired')}</FieldHint> : undefined}
          >
            <WizardTextField
              id={`${uid}-ssid`}
              value={n.ssid}
              icon={<Wifi size={15} className="pe-input__icon" aria-hidden="true" />}
              placeholder={AUTOCONFIG_PLACEHOLDERS.WIFI_SSID}
              invalid={ssidMissing}
              onChange={(ssid) => patch('network', { ssid })}
            />
          </WizardField>
          <WizardField label={t('settings.autoconfig.wizard.password')} htmlFor={`${uid}-key`} hint={wifiKeyHint(n.key, t)}>
            <PasswordInput id={`${uid}-key`} value={n.key} onChange={(key) => patch('network', { key })} />
          </WizardField>
        </div>
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
                invalid={!!ipErrors[field] || (field === 'ip' && addressMissing)}
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
