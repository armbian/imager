// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useId, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { X, FileText, ExternalLink, Calendar, Loader2, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getGithubRelease, openUrl } from '../../hooks/useTauri';
import type { GitHubRelease } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useDialogFocus } from '../../hooks/useDialogFocus';
import { ErrorDisplay } from './ErrorDisplay';
import { formatDate } from '../../utils';
import { releaseContributors, releaseNotesHtml } from '../../utils/releaseNotes';
import { LINKS } from '../../config';

interface ChangelogModalProps {
  isOpen: boolean;
  onClose: () => void;
  version: string;
}

// A GitHub release's notes and contributors; mounted only while open
export function ChangelogModal({ isOpen, onClose, version }: ChangelogModalProps) {
  if (!isOpen) return null;
  return <ChangelogDialog onClose={onClose} version={version} />;
}

function ChangelogDialog({ onClose, version }: Omit<ChangelogModalProps, 'isOpen'>) {
  const { t, i18n } = useTranslation();
  const titleId = useId();
  const dialogRef = useDialogFocus<HTMLDivElement>({ onEscape: onClose });
  const { data: release, loading, error, reload } = useAsyncData<GitHubRelease>(
    () => getGithubRelease(version),
    [version]
  );

  const releaseBody = release?.body?.trim() ? release.body : null;
  const html = useMemo(() => (releaseBody ? releaseNotesHtml(releaseBody) : ''), [releaseBody]);
  const contributors = useMemo(() => releaseContributors(releaseBody), [releaseBody]);

  // Portal to <body> so the fixed overlay escapes the animated settings shell
  return createPortal(
    <div className="changelog-modal-overlay" onClick={onClose}>
      <div
        ref={dialogRef}
        className="changelog-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="changelog-modal-header">
          <h2 id={titleId} className="changelog-modal-title">
            <span className="changelog-modal-title__icon" aria-hidden="true">
              <FileText size={16} />
            </span>
            <span>{t('update.changelogTitle')}</span>
          </h2>
          <button
            type="button"
            className="changelog-modal-close"
            onClick={onClose}
            aria-label={t('common.close')}
          >
            <X size={20} aria-hidden="true" />
          </button>
        </div>

        <div className="changelog-modal-content">
          {error ? (
            <div className="changelog-error">
              <ErrorDisplay error={error} onRetry={reload} compact />
              <p className="changelog-error-hint">{t('update.changelogErrorHint')}</p>
            </div>
          ) : loading || !release ? (
            <div className="changelog-loading">
              <Loader2 size={32} className="spinning" />
              <p>{t('update.loadingChangelog')}</p>
            </div>
          ) : (
            <>
              <div className="changelog-info">
                <h2 className="changelog-version">{release.name || release.tag_name}</h2>
                <div className="changelog-meta">
                  <span className="changelog-date">
                    <Calendar size={14} />
                    {formatDate(release.published_at, i18n.language, 'long')}
                  </span>
                  <button
                    type="button"
                    onClick={() => openUrl(release.html_url)}
                    className="changelog-link"
                  >
                    {t('update.viewOnGitHub')}
                    <ExternalLink size={14} />
                  </button>
                </div>
              </div>

              {html ? (
                <div className="changelog-body" dangerouslySetInnerHTML={{ __html: html }} />
              ) : (
                <p className="changelog-body">{t('update.noChangelog')}</p>
              )}

              {contributors.length > 0 && (
                <div className="changelog-contributors">
                  <div className="changelog-contributors-header">
                    <Users size={16} />
                    <span>{t('update.contributors')}</span>
                  </div>
                  <div className="changelog-contributors-list">
                    {contributors.map((username) => (
                      <button
                        key={username}
                        type="button"
                        className="changelog-contributor"
                        onClick={() => openUrl(`${LINKS.GITHUB}/${username}`)}
                        title={`@${username}`}
                      >
                        <img
                          src={`${LINKS.GITHUB}/${username}.png`}
                          alt={`@${username}`}
                          className="changelog-contributor-avatar"
                          loading="lazy"
                        />
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>,
    document.body
  );
}
