// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import {
  Bug,
  CircleArrowUp,
  CircleCheck,
  Download,
  ExternalLink,
  FileText,
  Github,
  Globe,
  Heart,
  Loader2,
  RefreshCw,
  ScrollText,
  TriangleAlert,
  Upload,
  Users,
  type LucideIcon,
} from 'lucide-react';
import { getTauriVersion, getSystemInfo, openUrl } from '../../hooks/useTauri';
import { getDeveloperMode, setDeveloperMode } from '../../hooks/useSettings';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useLogUpload } from '../../hooks/useLogUpload';
import { useSettingToggle } from '../../hooks/useSettingToggle';
import { useUpdate } from '../../contexts/UpdateContext';
import { ErrorDisplay } from '../shared/ErrorDisplay';
import { SettingsGroup, SettingsRow, ToggleRow } from './controls';
import { LogsModal } from './LogsModal';
import { LINKS, PLATFORM_LABEL, SETTINGS } from '../../config/constants';
import { displayUrl } from '../../utils';
import armbianLogo from '../../../src-tauri/icons/icon.png';

interface AboutLink {
  icon: LucideIcon;
  labelKey: string;
  url: string;
  subKey?: string;
}

const ABOUT_LINKS: readonly AboutLink[] = [
  { icon: Globe, labelKey: 'settings.about.website', url: LINKS.WEBSITE },
  { icon: ScrollText, labelKey: 'settings.documentation', url: LINKS.DOCS },
  { icon: Users, labelKey: 'settings.community', url: LINKS.FORUM },
  { icon: Github, labelKey: 'settings.githubRepo', url: LINKS.GITHUB_REPO },
  { icon: Bug, labelKey: 'settings.reportIssue', url: LINKS.ISSUES, subKey: 'settings.about.reportSub' },
  { icon: Heart, labelKey: 'settings.about.support', url: LINKS.DONATE },
];

async function openExternal(url: string) {
  try {
    await openUrl(url);
  } catch {
    window.open(url, '_blank');
  }
}

function UpdateStatus() {
  const { t } = useTranslation();
  const { update, available, open, recheck, checking, checkState } = useUpdate();

  if (checking) {
    return (
      <div className="about-page__status" role="status">
        <span className="about-page__status-text">
          <Loader2 size={16} className="spinning" aria-hidden="true" />
          {t('settings.about.checking')}
        </span>
      </div>
    );
  }

  if (available && update) {
    return (
      <div className="about-page__status" role="status">
        <span className="about-page__status-text is-new">
          <CircleArrowUp size={16} aria-hidden="true" />
          {t('settings.about.updateAvailable', { version: update.version })}
        </span>
        <button type="button" className="btn btn-primary btn-pill" onClick={open}>
          <Download size={16} aria-hidden="true" />
          {t('settings.about.updateRestart')}
        </button>
      </div>
    );
  }

  const status =
    checkState === 'failed' ? (
      <span className="about-page__status-text is-warn">
        <TriangleAlert size={16} aria-hidden="true" />
        {t('settings.about.checkFailed')}
      </span>
    ) : checkState === 'done' ? (
      <span className="about-page__status-text is-ok">
        <CircleCheck size={16} aria-hidden="true" />
        {t('settings.about.upToDate')}
      </span>
    ) : (
      <span className="about-page__status-text">{t('settings.about.notChecked')}</span>
    );

  return (
    <div className="about-page__status" role="status">
      {status}
      <button type="button" className="btn btn-secondary btn-pill" onClick={() => recheck()}>
        <RefreshCw size={16} aria-hidden="true" />
        {t(checkState === 'unchecked' ? 'settings.about.checkNow' : 'settings.about.checkAgain')}
      </button>
    </div>
  );
}

export function AboutSection() {
  const { t } = useTranslation();
  const [logsOpen, setLogsOpen] = useState(false);
  const { upload, uploading, url: pasteUrl, error: uploadError } = useLogUpload();
  const developerMode = useSettingToggle(getDeveloperMode, setDeveloperMode, { fallback: SETTINGS.DEFAULTS.DEVELOPER_MODE });
  const titleId = useId();
  const linksId = useId();

  const { data: appInfo, error, reload } = useAsyncData(async () => {
    const [appVersion, tauriVersion, systemInfo] = await Promise.all([getVersion(), getTauriVersion(), getSystemInfo()]);
    return {
      appVersion,
      tauriVersion,
      platform: PLATFORM_LABEL[systemInfo.platform] || systemInfo.platform,
      arch: systemInfo.arch,
    };
  }, []);

  return (
    <div className="about-page">
      <section className="about-page__hero" aria-labelledby={titleId}>
        <span className="about-page__icon">
          <img src={armbianLogo} alt="" />
        </span>
        <div className="about-page__hero-text">
          <h1 id={titleId} className="about-page__title">Armbian Imager</h1>
          {appInfo && (
            <p className="about-page__meta">
              <span className="about-page__version">v{appInfo.appVersion}</span>
              <span>
                {appInfo.platform} · {appInfo.arch}
              </span>
            </p>
          )}
          <UpdateStatus />
        </div>
      </section>

      {error && (
        <div className="about-page__error">
          <ErrorDisplay error={error} onRetry={reload} compact />
        </div>
      )}

      <section className="set-group" aria-labelledby={linksId}>
        <h3 id={linksId} className="set-group__eyebrow">
          {t('settings.links')}
        </h3>
        <div className="about-page__links">
          {ABOUT_LINKS.map(({ icon: Icon, labelKey, url, subKey }) => (
            <button key={url} type="button" className="about-link" onClick={() => openExternal(url)}>
              <span className="about-link__icon" aria-hidden="true">
                <Icon size={18} />
              </span>
              <span className="about-link__text">
                <b>{t(labelKey)}</b>
                <small>{subKey ? t(subKey) : displayUrl(url)}</small>
              </span>
              <ExternalLink className="about-link__arrow" size={16} aria-hidden="true" />
            </button>
          ))}
        </div>
      </section>

      <SettingsGroup eyebrow={t('settings.about.logsEyebrow')}>
        <SettingsRow
          label={t('settings.about.sessionLog')}
          description={t('settings.about.sessionLogDesc')}
          control={
            <span className="about-page__actions">
              <button type="button" className="btn btn-secondary btn-pill" onClick={() => setLogsOpen(true)}>
                <FileText size={16} aria-hidden="true" />
                {t('settings.about.open')}
              </button>
              <button
                type="button"
                className="btn btn-secondary btn-pill"
                onClick={() => upload()}
                disabled={uploading}
                aria-busy={uploading || undefined}
              >
                {uploading ? <Loader2 size={16} className="spinning" aria-hidden="true" /> : <Upload size={16} aria-hidden="true" />}
                {uploading ? t('errorDisplay.uploading') : t('settings.about.upload')}
              </button>
            </span>
          }
        />
        <SettingsRow
          label={t('settings.about.sharedLogs')}
          description={
            uploadError ? (
              <span className="about-page__upload-error" role="alert">
                {uploadError}
              </span>
            ) : (
              t('settings.about.sharedLogsDesc', { host: LINKS.PASTE_HOST })
            )
          }
          control={
            pasteUrl ? (
              <button type="button" className="about-page__paste" onClick={() => openExternal(pasteUrl)}>
                {displayUrl(pasteUrl)}
                <ExternalLink size={14} aria-hidden="true" />
              </button>
            ) : (
              <span className="about-page__value">{LINKS.PASTE_HOST}</span>
            )
          }
        />
      </SettingsGroup>

      {developerMode.value !== undefined && (
        <SettingsGroup eyebrow={t('settings.developer')}>
          <ToggleRow
            label={t('settings.developerMode')}
            description={t('settings.developerModeDescription')}
            checked={developerMode.value}
            onChange={(next) => developerMode.set(next)}
            busy={developerMode.busy}
          />
        </SettingsGroup>
      )}

      {appInfo && <p className="about-page__credits">{t('settings.about.credits', { tauri: appInfo.tauriVersion })}</p>}

      <LogsModal isOpen={logsOpen} onClose={() => setLogsOpen(false)} />
    </div>
  );
}
