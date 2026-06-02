import { useState, useEffect, useMemo } from 'react';
import { X, FileText, ExternalLink, Calendar, Loader2, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getGithubRelease, openUrl } from '../../hooks/useTauri';
import type { GitHubRelease } from '../../hooks/useTauri';
import { getErrorMessage } from '../../utils';

interface ChangelogModalProps {
  isOpen: boolean;
  onClose: () => void;
  version: string;
}

// Modal displaying a GitHub release's notes and changelog
export function ChangelogModal({ isOpen, onClose, version }: ChangelogModalProps) {
  const { t } = useTranslation();
  const [release, setRelease] = useState<GitHubRelease | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;

    const fetchRelease = async () => {
      setLoading(true);
      setError(null);

      try {
        const releaseData = await getGithubRelease(version);
        setRelease(releaseData);
      } catch (err) {
        console.error('Failed to fetch release:', err);
        setError(getErrorMessage(err, 'Failed to fetch changelog'));
      } finally {
        setLoading(false);
      }
    };

    fetchRelease();
  }, [isOpen, version]);

  // Unique @username mentions from the release body, sorted
  const contributors = useMemo(() => {
    if (!release?.body) return [];

    const mentions = release.body.match(/@([a-zA-Z0-9_-]+)/g);
    if (!mentions) return [];

    return Array.from(new Set(mentions.map(m => m.substring(1)))).sort();
  }, [release?.body]);

  if (!isOpen) return null;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  };

  // Convert the release body's markdown to HTML
  const parseReleaseBody = (body: string | null): string => {
    if (!body) return t('update.noChangelog', 'No changelog available');

    // Strip the "Full Changelog" line GitHub appends
    let cleanedBody = body.replace(/\*\*Full Changelog\*\*: https:\/\/github\.com\/[^\s]+/gi, '');
    cleanedBody = cleanedBody.replace(/Full Changelog: https:\/\/github\.com\/[^\s]+/gi, '');

    const lines = cleanedBody.split('\n');
    let html = '';
    let inList = false;
    let inCodeBlock = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      if (line.trim().startsWith('```')) {
        inCodeBlock = !inCodeBlock;
        continue;
      }

      if (inCodeBlock) {
        html += `<div class="code-block">${line}</div>`;
        continue;
      }

      const isListItem = /^\s*[-*]\s+/.test(line);

      if (isListItem) {
        if (!inList) {
          html += '<ul>';
          inList = true;
        }

        const itemContent = line.replace(/^\s*[-*]\s+/, '');
        const formattedItem = itemContent
          .replace(/ https:\/\/github\.com\/[^/]+\/[^/]+\/(?:pull|issues)\/(\d+)/g, ' #PR$1 ')
          .replace(/\[[^\]]+\]\(https:\/\/github\.com\/[^/]+\/[^/]+\/(?:pull|issues)\/(\d+)\)/g, '#PR$1')
          // Escape only after PR placeholders are inserted, so the markup survives
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;')
          .replace(/#PR(\d+)/g, '<span class="pr-number">#$1</span>')
          .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
          .replace(/\*([^*]+)\*/g, '<em>$1</em>')
          .replace(/`([^`]+)`/g, '<code>$1</code>')
          .replace(/\[([^\]]+)\]\(https:\/\/github\.com\/[^)]+\)/g, '$1')
          .replace(/https:\/\/github\.com\/[^\s]+/g, '')
          .replace(/,\s*$/g, '')
          .replace(/\s+/g, ' ')
          .trim()
          // Keep non-GitHub external links
          .replace(/\[([^\]]+)\]\((?!https?:\/\/github\.com)(https?:\/\/[^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');

        html += `<li>${formattedItem}</li>`;
      } else {
        if (inList) {
          html += '</ul>';
          inList = false;
        }

        if (line.trim() === '') {
          html += '<br />';
          continue;
        }

        if (line.startsWith('### ')) {
          html += `<h3>${line.slice(4)}</h3>`;
          continue;
        }

        if (line.startsWith('## ')) {
          html += `<h2>${line.slice(3)}</h2>`;
          continue;
        }

        const formattedLine = line
          .replace(/ https:\/\/github\.com\/[^/]+\/[^/]+\/(?:pull|issues)\/(\d+)/g, ' #PR$1 ')
          .replace(/\[[^\]]+\]\(https:\/\/github\.com\/[^/]+\/[^/]+\/(?:pull|issues)\/(\d+)\)/g, '#PR$1')
          // Escape only after PR placeholders are inserted, so the markup survives
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;')
          .replace(/#PR(\d+)/g, '<span class="pr-number">#$1</span>')
          .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
          .replace(/\*([^*]+)\*/g, '<em>$1</em>')
          .replace(/`([^`]+)`/g, '<code>$1</code>')
          .replace(/\[([^\]]+)\]\(https:\/\/github\.com\/[^)]+\)/g, '$1')
          .replace(/https:\/\/github\.com\/[^\s]+/g, '')
          .replace(/,\s*$/g, '')
          .replace(/\s+/g, ' ')
          .trim()
          // Keep non-GitHub external links
          .replace(/\[([^\]]+)\]\((?!https?:\/\/github\.com)(https?:\/\/[^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');

        html += `<p>${formattedLine}</p>`;
      }
    }

    if (inList) {
      html += '</ul>';
    }

    // Collapse stray/duplicate <br /> tags to avoid extra spacing
    html = html.replace(/(<br\s*\/?>)+$/g, '');
    html = html.replace(/(<br\s*\/?>){2,}/g, '<br />');
    html = html.replace(/<br\s*\/?>\s*<h2>/g, '<h2>');
    html = html.replace(/<br\s*\/?>\s*<h3>/g, '<h3>');

    return html;
  };

  return (
    <div className="changelog-modal-overlay" onClick={onClose}>
      <div
        className="changelog-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="changelog-modal-header">
          <div className="changelog-modal-title">
            <span className="changelog-modal-title__icon">
              <FileText size={16} />
            </span>
            <span>{t('update.changelogTitle', 'What\'s New')}</span>
          </div>
          <button
            className="changelog-modal-close"
            onClick={onClose}
            aria-label="Close"
          >
            <X size={20} />
          </button>
        </div>

        <div className="changelog-modal-content">
          {loading ? (
            <div className="changelog-loading">
              <Loader2 size={32} className="spinning" />
              <p>{t('update.loadingChangelog', 'Loading changelog...')}</p>
            </div>
          ) : error ? (
            <div className="changelog-error">
              <p>{error}</p>
              <p className="changelog-error-hint">
                {t('update.changelogErrorHint', 'Make sure you have an internet connection and the version exists on GitHub.')}
              </p>
            </div>
          ) : release ? (
            <>
              <div className="changelog-info">
                <h2 className="changelog-version">{release.name || release.tag_name}</h2>
                <div className="changelog-meta">
                  <span className="changelog-date">
                    <Calendar size={14} />
                    {formatDate(release.published_at)}
                  </span>
                  <button
                    onClick={() => openUrl(release.html_url)}
                    className="changelog-link"
                  >
                    {t('update.viewOnGitHub', 'View on GitHub')}
                    <ExternalLink size={14} />
                  </button>
                </div>
              </div>

              <div
                className="changelog-body"
                dangerouslySetInnerHTML={{
                  __html: parseReleaseBody(release.body),
                }}
              />

              {contributors.length > 0 && (
                <div className="changelog-contributors">
                  <div className="changelog-contributors-header">
                    <Users size={16} />
                    <span>{t('update.contributors', 'Contributors')}</span>
                  </div>
                  <div className="changelog-contributors-list">
                    {contributors.map((username) => (
                      <button
                        key={username}
                        className="changelog-contributor"
                        onClick={() => openUrl(`https://github.com/${username}`)}
                        title={`@${username}`}
                      >
                        <img
                          src={`https://github.com/${username}.png`}
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
          ) : null}
        </div>
      </div>
    </div>
  );
}
