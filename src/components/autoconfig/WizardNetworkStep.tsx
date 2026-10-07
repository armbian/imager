// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Cable, CircleCheck, Globe, TriangleAlert, Wifi } from 'lucide-react';
import { UI } from '../../config';
import { AUTOCONFIG_PLACEHOLDERS, WIFI_COUNTRY_CODES, isWifiKeyLength } from '../../config/autoconfig';
import { WIZARD_ERROR, hasWifi } from '../../config/profileWizard';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { PasswordInput } from '../shared/PasswordInput';
import { SearchPicker, type PickerOption } from '../shared/SearchPicker';
import { SegmentedChoice } from '../shared/SegmentedChoice';
import { CountryFlag, WizardField, WizardMore, WizardTextField } from './WizardFields';

const SEG_ICON = UI.ICON_SIZE.WIZARD_SEG;

function WifiKeyFit({ wifiKey }: { wifiKey: string }) {
  const { t } = useTranslation();
  if (wifiKey === '') return null;
  const fits = isWifiKeyLength(wifiKey);
  const label = t(fits ? 'settings.autoconfig.wifiKeyFits' : 'settings.autoconfig.wifiKeyLength');
  return (
    <span className={`pw-fit ${fits ? 'is-ok' : 'is-warn'}`} role="img" aria-label={label} title={label}>
      {fits ? <CircleCheck size={15} /> : <TriangleAlert size={15} />}
    </span>
  );
}

export function WizardNetworkStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t } = useTranslation();
  const { draft, more, attempted, stepError, ipErrors, ipMessages, addressMissing, patch, toggleMore } = wizard;
  const n = draft.network;
  const wifi = hasWifi(n.mode);
  const ssidMissing = attempted && stepError === WIZARD_ERROR.SSID;
  const countries = useMemo<PickerOption[]>(
    () => WIFI_COUNTRY_CODES.map((c) => ({ value: c.code, label: c.name, icon: <CountryFlag code={c.code} /> })),
    []
  );

  return (
    <>
      <SegmentedChoice
        value={n.mode}
        ariaLabel={t('settings.autoconfig.wizard.network.title')}
        options={[
          { value: 'cable', label: t('settings.autoconfig.wizard.network.cable'), icon: <Cable size={SEG_ICON} aria-hidden="true" /> },
          { value: 'wifi', label: t('settings.autoconfig.wizard.network.wifi'), icon: <Wifi size={SEG_ICON} aria-hidden="true" /> },
          {
            value: 'both',
            label: t('settings.autoconfig.wizard.network.both'),
            icon: (
              <span className="seg-choice__icons" aria-hidden="true">
                <Cable size={SEG_ICON} />
                <Wifi size={SEG_ICON} />
              </span>
            ),
          },
        ]}
        onChange={(mode) => patch('network', { mode })}
      />
      {wifi ? (
        <div className="pw-two">
          <WizardField label={t('settings.autoconfig.wizard.network.ssid')} htmlFor={`${uid}-ssid`} error={ssidMissing ? t(WIZARD_ERROR.SSID) : null}>
            <WizardTextField
              id={`${uid}-ssid`}
              value={n.ssid}
              icon={<Wifi size={15} className="ac-input__icon" aria-hidden="true" />}
              placeholder={AUTOCONFIG_PLACEHOLDERS.WIFI_SSID}
              invalid={ssidMissing}
              onChange={(ssid) => patch('network', { ssid })}
            />
          </WizardField>
          <WizardField label={t('settings.autoconfig.wizard.password')} htmlFor={`${uid}-key`}>
            <PasswordInput
              id={`${uid}-key`}
              value={n.key}
              onChange={(key) => patch('network', { key })}
              trailing={<WifiKeyFit wifiKey={n.key} />}
            />
          </WizardField>
        </div>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.network.cableLine')}</p>
      )}
      <WizardMore
        label={t(wifi ? 'settings.autoconfig.wizard.network.moreWifi' : 'settings.autoconfig.wizard.network.moreCable')}
        open={!!more.network}
        onToggle={() => toggleMore('network')}
      />
      {more.network && (
        <div className="pw-two">
          {wifi && (
            <WizardField label={t('settings.autoconfig.wizard.network.country')}>
              <SearchPicker
                value={n.country}
                options={countries}
                onChange={(country) => patch('network', { country })}
                label={t('settings.autoconfig.wizard.network.country')}
                searchPlaceholder={t('settings.autoconfig.wizard.network.searchCountry')}
                placeholder={t('settings.autoconfig.selectPlaceholder')}
                icon={<Globe size={15} className="ac-input__icon" aria-hidden="true" />}
              />
            </WizardField>
          )}
          <WizardField label={t('settings.autoconfig.wizard.network.ip')}>
            <SegmentedChoice
              size="sm"
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
      {more.network && n.ip === 'fixed' && (
        <div className="pw-four">
          <WizardField label={t('settings.autoconfig.wizard.network.address')} htmlFor={`${uid}-ip`}>
            <WizardTextField id={`${uid}-ip`} mono value={n.address} placeholder={AUTOCONFIG_PLACEHOLDERS.STATIC_IP} invalid={!!ipErrors.ip || !!addressMissing} onChange={(address) => patch('network', { address })} />
          </WizardField>
          <WizardField label={t('settings.autoconfig.wizard.network.netmask')} htmlFor={`${uid}-mask`}>
            <WizardTextField id={`${uid}-mask`} mono value={n.mask} placeholder={AUTOCONFIG_PLACEHOLDERS.STATIC_MASK} invalid={!!ipErrors.mask} onChange={(mask) => patch('network', { mask })} />
          </WizardField>
          <WizardField label={t('settings.autoconfig.wizard.network.gateway')} htmlFor={`${uid}-gw`}>
            <WizardTextField id={`${uid}-gw`} mono value={n.gateway} placeholder={AUTOCONFIG_PLACEHOLDERS.STATIC_GATEWAY} invalid={!!ipErrors.gateway} onChange={(gateway) => patch('network', { gateway })} />
          </WizardField>
          <WizardField label={t('settings.autoconfig.wizard.network.dns')} htmlFor={`${uid}-dns`}>
            <WizardTextField id={`${uid}-dns`} mono value={n.dns} placeholder={AUTOCONFIG_PLACEHOLDERS.STATIC_DNS} invalid={!!ipErrors.dns} onChange={(dns) => patch('network', { dns })} />
          </WizardField>
          {ipMessages.length > 0 && (
            // Four columns are too narrow for the messages, so they run under the whole row.
            <div className="pw-four__msgs" role="alert">
              {ipMessages.map((message) => (
                <div key={message} className="pw-hint pw-hint--line is-error">
                  <TriangleAlert size={UI.ICON_SIZE.WIZARD_HINT} aria-hidden="true" />
                  {message}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </>
  );
}
