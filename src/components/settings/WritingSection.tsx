// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowRight, Download, HardDrive, Info, ShieldCheck } from 'lucide-react';
import { PLATFORM, SETTINGS } from '../../config';
import { DEV_ARMBIAN_HOST } from '../../config/devScenarios';
import {
  getAllowSystemDevices,
  setAllowSystemDevices,
  getArmbianBoardDetection,
  setArmbianBoardDetection,
  getSkipVerify,
  setSkipVerify,
} from '../../hooks/useSettings';
import { getArmbianRelease, getSystemInfo } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useSettingToggle } from '../../hooks/useSettingToggle';
import { SettingsHero } from './SettingsHero';
import { SegmentedControl, SettingsGroup, SettingsRow, ToggleRow } from './controls';

const DETECTION_OPTIONS = [
  { value: SETTINGS.ARMBIAN_DETECTION_MODES.DISABLED, labelKey: 'settings.armbian.mode_disabled' },
  { value: SETTINGS.ARMBIAN_DETECTION_MODES.MODAL, labelKey: 'settings.armbian.mode_modal' },
  { value: SETTINGS.ARMBIAN_DETECTION_MODES.AUTO, labelKey: 'settings.armbian.mode_auto' },
] as const;

async function hostRunsArmbian(): Promise<boolean> {
  const info = await getSystemInfo();
  // Debug builds ask on every platform: the dev scenarios panel can simulate an Armbian host
  if (info.platform !== PLATFORM.LINUX && !__DEV_SCENARIOS__) return false;
  return (await getArmbianRelease()) !== null;
}

function FlowChips({ verify }: { verify: boolean }) {
  const { t } = useTranslation();
  const arrow = <ArrowRight size={14} className="writing-flow__arrow" aria-hidden="true" />;
  return (
    <span className="writing-flow">
      <span className="writing-flow__chip">
        <Download size={14} aria-hidden="true" />
        {t('settings.writing.chipDownload')}
      </span>
      {arrow}
      <span className="writing-flow__chip">
        <HardDrive size={14} aria-hidden="true" />
        {t('settings.writing.chipWrite')}
      </span>
      <span className={`writing-flow__tail${verify ? '' : ' is-off'}`} aria-hidden={!verify}>
        {arrow}
        <span className="writing-flow__chip writing-flow__chip--ok">
          <ShieldCheck size={14} aria-hidden="true" />
          {t('settings.writing.chipVerify')}
        </span>
      </span>
    </span>
  );
}

export function WritingSection() {
  const { t } = useTranslation();
  const skipVerify = useSettingToggle(getSkipVerify, setSkipVerify, {
    fallback: SETTINGS.DEFAULTS.SKIP_VERIFY,
    toastKey: 'settings.toast.skipVerify',
  });
  const detection = useSettingToggle(getArmbianBoardDetection, setArmbianBoardDetection, {
    fallback: SETTINGS.DEFAULTS.ARMBIAN_BOARD_DETECTION,
    toastKey: 'settings.toast.detection',
  });
  const systemDrives = useSettingToggle(getAllowSystemDevices, setAllowSystemDevices, {
    fallback: SETTINGS.DEFAULTS.ALLOW_SYSTEM_DEVICES,
    toastKey: 'settings.toast.allowSystemDevices',
  });
  const { data: isArmbian, loading: checkingHost, reload: recheckHost } = useAsyncData(hostRunsArmbian, []);

  useEffect(() => {
    if (!__DEV_SCENARIOS__) return;
    window.addEventListener(DEV_ARMBIAN_HOST.CHANGED_EVENT, recheckHost);
    return () => window.removeEventListener(DEV_ARMBIAN_HOST.CHANGED_EVENT, recheckHost);
  }, [recheckHost]);

  const verifyLoaded = skipVerify.value !== undefined;
  const verify = verifyLoaded && !skipVerify.value;
  const detectionAvailable = isArmbian === true;

  return (
    <>
      <SettingsHero
        hero="writing"
        title={t('settings.nav.writing')}
        lead={t('settings.writing.lead')}
        heroState={{ verify: verify ? 'on' : 'off' }}
        loading={!verifyLoaded}
      />

      <SettingsGroup eyebrow={t('settings.writing.whatHappens')}>
        <div className={verifyLoaded ? undefined : 'writing-flow-row is-pending'} aria-hidden={verifyLoaded ? undefined : true}>
          <SettingsRow
            label={t(verify ? 'settings.writing.flowVerify' : 'settings.writing.flowNoVerify')}
            description={t(verify ? 'settings.writing.flowVerifyDesc' : 'settings.writing.flowNoVerifyDescNoReadback')}
            control={<FlowChips verify={verify || !verifyLoaded} />}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup eyebrow={t('settings.nav.writing')}>
        <ToggleRow
          label={t('settings.writing.verifyAfter')}
          description={t('settings.writing.verifyAfterDesc')}
          checked={verify}
          loading={skipVerify.value === undefined}
          busy={skipVerify.busy}
          onChange={(on) => skipVerify.set(!on)}
        />
        <SettingsRow
          label={t('settings.armbian.label')}
          description={
            <span className="writing-desc">
              <span>{t('settings.armbian.description')}</span>
              {!checkingHost && !detectionAvailable && <span className="writing-desc__hint">
                  <Info size={12} aria-hidden="true" />
                  {t('settings.writing.detectionUnavailable')}
                </span>}
            </span>
          }
          control={
            <SegmentedControl<string>
              value={detectionAvailable ? (detection.value ?? SETTINGS.ARMBIAN_DETECTION_MODES.DISABLED) : SETTINGS.ARMBIAN_DETECTION_MODES.DISABLED}
              ariaLabel={t('settings.armbian.label')}
              disabled={!detectionAvailable}
              loading={detection.value === undefined}
              onChange={detection.set}
              options={DETECTION_OPTIONS.map(({ value, labelKey }) => ({ value, label: t(labelKey) }))}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup eyebrow={t('settings.writing.dangerZone')} danger>
        <ToggleRow
          label={t('settings.allowSystemDevices')}
          description={t('settings.allowSystemDevicesDescription')}
          checked={systemDrives.value ?? false}
          loading={systemDrives.value === undefined}
          busy={systemDrives.busy}
          onChange={systemDrives.set}
          danger
        />
      </SettingsGroup>
    </>
  );
}
