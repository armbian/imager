// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Cable, Network, Wifi } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, WIFI_COUNTRY_CODES, isWifiKeyLength } from '../../../config/autoconfig';
import {
  PROFILE_EDITOR,
  countryName,
  editorIpErrors,
  hasWifi,
  type IpMode,
  type NetMode,
} from '../../../config/profileEditorModel';
import type { StaticIpErrors } from '../../../utils';
import { PasswordInput } from '../../shared/PasswordInput';
import { SegmentedControl, SelectMenu, SettingsRow, type SelectOption } from '../controls';
import { CountryFlag, DefaultsAction, EditorSection, FieldHint, MoreRow, TextField } from './EditorParts';
import { fieldId, type FoldingGroupProps } from './types';

const IP_FIELDS: { field: keyof StaticIpErrors; key: 'address' | 'mask' | 'gateway' | 'dns'; label: string; placeholder: string }[] = [
  { field: 'ip', key: 'address', label: 'address', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_IP },
  { field: 'mask', key: 'mask', label: 'netmask', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_MASK },
  { field: 'gateway', key: 'gateway', label: 'gateway', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_GATEWAY },
  { field: 'dns', key: 'dns', label: 'dns', placeholder: AUTOCONFIG_PLACEHOLDERS.STATIC_DNS },
];

export function NetworkGroup({ model, status, uid, patch, keepDefaults, more, toggleMore }: FoldingGroupProps) {
  const { t, i18n } = useTranslation();
  const n = model.network;
  const wifi = n.apply && hasWifi(n.mode);
  const both = n.mode === 'both';
  const ipErrors = editorIpErrors(n);
  const subId = fieldId(uid, 'network-more');

  const countries = useMemo<SelectOption<string>[]>(
    () => [
      { value: '', label: t('settings.autoconfig.selectPlaceholder') },
      ...WIFI_COUNTRY_CODES.map((c) => ({
        value: c.code,
        label: countryName(c.code, c.name, i18n.language),
        icon: <CountryFlag code={c.code} />,
        badge: c.code,
      })),
    ],
    [t, i18n.language]
  );

  const keyHint = !n.key ? (
    <FieldHint>{t('settings.autoconfig.editor.wifiKeyEmpty')}</FieldHint>
  ) : isWifiKeyLength(n.key) ? (
    <FieldHint tone="ok">{t('settings.autoconfig.wifiKeyFits')}</FieldHint>
  ) : (
    <FieldHint tone="warn">{t('settings.autoconfig.wifiKeyLength')}</FieldHint>
  );

  const summary = [
    n.ip === 'fixed'
      ? n.address.trim()
        ? t('settings.autoconfig.editor.netFixed', { ip: n.address.trim() })
        : t('settings.autoconfig.wizard.network.ipFixed')
      : t('settings.autoconfig.editor.netAuto'),
    wifi && n.country,
  ]
    .filter(Boolean)
    .join(PROFILE_EDITOR.SEPARATOR);

  const ipHint = (field: keyof StaticIpErrors) => {
    const error = ipErrors[field];
    if (error) return <FieldHint tone="warn">{t(`settings.autoconfig.${error}`)}</FieldHint>;
    if (field !== 'ip') return undefined;
    if (!n.address.trim()) return <FieldHint tone="warn">{t('settings.autoconfig.wizard.addressRequired')}</FieldHint>;
    return <FieldHint tone="ok">{t('settings.autoconfig.editor.addressOk')}</FieldHint>;
  };

  return (
    <EditorSection
      id="network"
      eyebrow={t('settings.autoconfig.groupNetwork')}
      description={t('settings.autoconfig.editor.groupNetworkDesc')}
      attention={status === 'attn'}
      action={n.apply ? <DefaultsAction onClick={(byPointer) => keepDefaults('network', byPointer)} /> : undefined}
    >
      <SettingsRow
        label={t('settings.autoconfig.editor.connection')}
        tall={!wifi}
        description={
          wifi ? undefined : (
            <FieldHint>{t(n.apply ? 'settings.autoconfig.editor.cableHint' : 'settings.autoconfig.armbianDefaults')}</FieldHint>
          )
        }
        control={
          <SegmentedControl<NetMode>
            value={n.apply ? n.mode : null}
            ariaLabel={t('settings.autoconfig.editor.connection')}
            options={[
              { value: 'cable', label: t('settings.autoconfig.wizard.network.cable'), icon: <Cable aria-hidden="true" /> },
              { value: 'wifi', label: t('settings.autoconfig.wizard.network.wifi'), icon: <Wifi aria-hidden="true" /> },
              { value: 'both', label: t('settings.autoconfig.wizard.network.both'), icon: <Network aria-hidden="true" /> },
            ]}
            onChange={(mode) => patch('network', { mode })}
          />
        }
      />
      {wifi && (
        <>
          <SettingsRow
            label={t('settings.autoconfig.wizard.network.ssid')}
            controlId={fieldId(uid, 'ssid')}
            tall={!n.ssid.trim()}
            description={n.ssid.trim() ? undefined : <FieldHint tone="warn">{t('settings.autoconfig.wizard.ssidRequired')}</FieldHint>}
            control={
              <TextField
                id={fieldId(uid, 'ssid')}
                value={n.ssid}
                icon={<Wifi size={16} className="pe-input__icon" aria-hidden="true" />}
                placeholder={AUTOCONFIG_PLACEHOLDERS.WIFI_SSID}
                invalid={!n.ssid.trim()}
                onChange={(ssid) => patch('network', { ssid })}
              />
            }
          />
          <SettingsRow
            label={t('settings.autoconfig.wizard.password')}
            controlId={fieldId(uid, 'wifi-key')}
            tall
            description={keyHint}
            control={<PasswordInput id={fieldId(uid, 'wifi-key')} value={n.key} onChange={(key) => patch('network', { key })} />}
          />
        </>
      )}
      {n.apply && (
        <MoreRow
          label={t('settings.autoconfig.editor.moreOptions')}
          value={summary}
          open={more}
          controls={subId}
          onToggle={() => toggleMore('network')}
        />
      )}
      <div id={subId} className="pe-sub" hidden={!n.apply || !more}>
        {wifi && (
          <SettingsRow
            label={t('settings.autoconfig.wizard.network.country')}
            tall
            description={<FieldHint>{t('settings.autoconfig.editor.countryHint')}</FieldHint>}
            control={
              <SelectMenu
                value={n.country}
                options={countries}
                onChange={(country) => patch('network', { country })}
                ariaLabel={t('settings.autoconfig.wizard.network.country')}
                searchable
                searchPlaceholder={t('settings.autoconfig.wizard.network.searchCountry')}
              />
            }
          />
        )}
        <SettingsRow
          label={t(both ? 'settings.autoconfig.editor.ipLabelWifi' : 'settings.autoconfig.wizard.network.ip')}
          tall
          description={
            <FieldHint>
              {t(
                both
                  ? 'settings.autoconfig.editor.bothCableAuto'
                  : n.ip === 'fixed'
                    ? 'settings.autoconfig.editor.ipFixedHint'
                    : 'settings.autoconfig.editor.ipAutoHint'
              )}
            </FieldHint>
          }
          control={
            <SegmentedControl<IpMode>
              value={n.ip}
              ariaLabel={t('settings.autoconfig.wizard.network.ip')}
              options={[
                { value: 'auto', label: t('settings.autoconfig.wizard.network.ipAuto') },
                { value: 'fixed', label: t('settings.autoconfig.wizard.network.ipFixed') },
              ]}
              onChange={(ip) => patch('network', { ip })}
            />
          }
        />
        {n.ip === 'fixed' &&
          IP_FIELDS.map(({ field, key, label, placeholder }) => {
            const id = fieldId(uid, `ip-${key}`);
            const hint = ipHint(field);
            return (
              <SettingsRow
                key={key}
                label={t(`settings.autoconfig.wizard.network.${label}`)}
                controlId={id}
                tall={!!hint}
                description={hint}
                control={
                  <TextField
                    id={id}
                    mono
                    value={n[key]}
                    placeholder={placeholder}
                    invalid={!!ipErrors[field]}
                    onChange={(value) => patch('network', { [key]: value })}
                  />
                }
              />
            );
          })}
      </div>
    </EditorSection>
  );
}
