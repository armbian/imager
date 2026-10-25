// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import {
  Bug,
  ChevronRight,
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
import { getTauriVersion, getSystemInfo, openExternal } from '../../hooks/useTauri';
import { getDeveloperMode, setDeveloperMode } from '../../hooks/useSettings';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useLogUpload } from '../../hooks/useLogUpload';
import { useSettingToggle } from '../../hooks/useSettingToggle';
import { useUpdate } from '../../contexts/UpdateContext';
import { ChangelogModal, ErrorDisplay, UpdateProgressBar } from '../shared';
import { SettingsGroup, SettingsRow, ToggleRow } from './controls';
import { LogsModal } from './LogsModal';
import { LINKS, PLATFORM_LABEL, SETTINGS, UI } from '../../config/constants';
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

function UpdateStatus() {
  const { t } = useTranslation();
  const { recheck, checking, checkState } = useUpdate();

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

function UpdateStrip({ version }: { version: string }) {
  const { t } = useTranslation();
  const { status, progress, errorMessage, failedAt, restartBlocked, download, restart, retry } = useUpdate();
  const [notesOpen, setNotesOpen] = useState(false);

  const notesLink = (
    <button type="button" className="about-page__notes-link" onClick={() => setNotesOpen(true)}>
      {t('update.viewChangelog')}
      <ChevronRight size={UI.ICON_SIZE.CHEVRON} aria-hidden="true" />
    </button>
  );

  let tone = '';
  let content;
  if (status === 'downloading') {
    content = (
      <div className="about-update__progress">
        <UpdateProgressBar progress={progress} variant="detail" label={t('update.downloadingVersion', { version })} />
      </div>
    );
  } else if (status === 'ready' || status === 'installing') {
    const installing = status === 'installing';
    const note = restartBlocked ? t('update.restartBlocked') : t('update.versionReady', { version });
    tone = ' is-ready';
    content = (
      <>
        <span className="about-update__text" role="status" title={note}>
          {note}
        </span>
        {notesLink}
        <button type="button" className="btn btn-primary btn-pill" onClick={restart} disabled={installing || restartBlocked}>
          <RefreshCw size={UI.ICON_SIZE.BUTTON} className={installing ? 'spinning' : undefined} aria-hidden="true" />
          {t(installing ? 'update.restarting' : 'update.restartNow')}
        </button>
      </>
    );
  } else if (status === 'error') {
    const retryBlocked = restartBlocked && failedAt !== 'download';
    const message = retryBlocked ? t('update.restartBlocked') : errorMessage || t('update.errorMessage');
    tone = ' is-error';
    content = (
      <>
        <span className="about-update__text" role="alert" title={message}>
          {message}
        </span>
        <button type="button" className="btn btn-secondary btn-pill" onClick={retry} disabled={retryBlocked}>
          <RefreshCw size={UI.ICON_SIZE.BUTTON} aria-hidden="true" />
          {t('update.retry')}
        </button>
      </>
    );
  } else {
    const text = t('update.appVersionAvailable', { version });
    content = (
      <>
        <span className="about-update__text" role="status" title={text}>
          {text}
        </span>
        {notesLink}
        <button type="button" className="btn btn-primary btn-pill" onClick={download}>
          <Download size={UI.ICON_SIZE.BUTTON} aria-hidden="true" />
          {t('update.install')}
        </button>
      </>
    );
  }

  return (
    <div className={`about-update${tone}`}>
      {content}
      <ChangelogModal isOpen={notesOpen} onClose={() => setNotesOpen(false)} version={version} />
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
  const { update } = useUpdate();

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
      <div className={`about-page__head${update ? ' has-update' : ''}`}>
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
            {!update && <UpdateStatus />}
          </div>
        </section>
        {update && <UpdateStrip version={update.version} />}
      </div>

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
