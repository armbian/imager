import { useState, useEffect, useMemo } from 'react';
import { X, FileText, ExternalLink, Calendar, Loader2, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getGithubRelease, openUrl } from '../../hooks/useTauri';
import type { GitHubRelease } from '../../hooks/useTauri';

interface ChangelogModalProps {
  isOpen: boolean;
  onClose: () => void;
  version: string;
}

/**
 * Modal component to display GitHub release changelog
 *
 * Shows release notes, published date, and link to full release on GitHub
 */
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
        setError(err instanceof Error ? err.message : 'Failed to fetch changelog');
      } finally {
        setLoading(false);
      }
    };

    fetchRelease();
  }, [isOpen, version]);

  /**
   * Extract unique contributor usernames from the release body
   * GitHub releases typically format contributors as "by @username"
   */
  const contributors = useMemo(() => {
    if (!release?.body) return [];

    // Match all @username mentions
    const mentions = release.body.match(/@([a-zA-Z0-9_-]+)/g);
    if (!mentions) return [];

    // Extract unique usernames, remove duplicates, and sort alphabetically
    const uniqueUsernames = Array.from(
      new Set(mentions.map(m => m.substring(1))) // Remove @ symbol
    ).sort();

    return uniqueUsernames;
  }, [release?.body]);

  if (!isOpen) return null;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  };

  /**
   * Parse release body and convert markdown-style formatting to HTML
   * Handles:
   * - Headers (##, ###)
   * - Lists (-, *)
   * - Links [text](url)
   * - Inline code
   * - Bold text (**text**)
   * - Italic text (*text*)
   * - Line breaks and paragraphs
   */
  const parseReleaseBody = (body: string | null): string => {
    if (!body) return t('update.noChangelog', 'No changelog available');

    // Remove "Full Changelog" section that GitHub adds at the end
    let cleanedBody = body.replace(/\*\*Full Changelog\*\*: https:\/\/github\.com\/[^\s]+/gi, '');
    cleanedBody = cleanedBody.replace(/Full Changelog: https:\/\/github\.com\/[^\s]+/gi, '');

    // Split into lines for better list processing
    const lines = cleanedBody.split('\n');
    let html = '';
    let inList = false;
    let inCodeBlock = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Check for code blocks
      if (line.trim().startsWith('```')) {
        inCodeBlock = !inCodeBlock;
        continue;
      }

      if (inCodeBlock) {
        html += `<div class="code-block">${line}</div>`;
        continue;
      }

      // Check for list items
      const isListItem = /^\s*[-*]\s+/.test(line);

      if (isListItem) {
        if (!inList) {
          html += '<ul>';
          inList = true;
        }

        const itemContent = line.replace(/^\s*[-*]\s+/, '');
        const formattedItem = itemContent
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;')
          .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
          .replace(/\*([^*]+)\*/g, '<em>$1</em>')
          .replace(/`([^`]+)`/g, '<code>$1</code>')
          // Remove markdown GitHub links (PRs, issues, commits)
          .replace(/\[([^\]]+)\]\(https:\/\/github\.com\/[^)]+\)/g, '$1')
          // Remove "in https://github.com/..." but keep "by @username"
          .replace(/(by @[a-zA-Z0-9_-]+) in https:\/\/github\.com\/[^\s]+/g, '$1')
          // Remove any remaining bare GitHub URLs
          .replace(/https:\/\/github\.com\/[^\s]+/g, '')
          .replace(/,\s*$/g, '') // Remove trailing comma after URL removal
          .replace(/\s+/g, ' ') // Clean up extra whitespace
          .trim()
          // Keep only non-GitHub external links (markdown format)
          .replace(/\[([^\]]+)\]\((?!https?:\/\/github\.com)(https?:\/\/[^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');

        html += `<li>${formattedItem}</li>`;
      } else {
        if (inList) {
          html += '</ul>';
          inList = false;
        }

        // Handle empty lines as paragraph breaks
        if (line.trim() === '') {
          html += '<br />';
          continue;
        }

        // Handle headers
        if (line.startsWith('### ')) {
          html += `<h3>${line.slice(4)}</h3>`;
          continue;
        }

        if (line.startsWith('## ')) {
          html += `<h2>${line.slice(3)}</h2>`;
          continue;
        }

        // Regular paragraph line
        const formattedLine = line
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;')
          .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
          .replace(/\*([^*]+)\*/g, '<em>$1</em>')
          .replace(/`([^`]+)`/g, '<code>$1</code>')
          // Remove markdown GitHub links (PRs, issues, commits)
          .replace(/\[([^\]]+)\]\(https:\/\/github\.com\/[^)]+\)/g, '$1')
          // Remove "in https://github.com/..." but keep "by @username"
          .replace(/(by @[a-zA-Z0-9_-]+) in https:\/\/github\.com\/[^\s]+/g, '$1')
          // Remove any remaining bare GitHub URLs
          .replace(/https:\/\/github\.com\/[^\s]+/g, '')
          .replace(/,\s*$/g, '') // Remove trailing comma after URL removal
          .replace(/\s+/g, ' ') // Clean up extra whitespace
          .trim()
          // Keep only non-GitHub external links (markdown format)
          .replace(/\[([^\]]+)\]\((?!https?:\/\/github\.com)(https?:\/\/[^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');

        html += `<p>${formattedLine}</p>`;
      }
    }

    // Close unclosed list
    if (inList) {
      html += '</ul>';
    }

    // Remove trailing <br /> tags to avoid extra space at the end
    html = html.replace(/(<br\s*\/?>)+$/g, '');

    return html;
  };

  return (
    <div className="changelog-modal-overlay" onClick={onClose}>
      <div
        className="changelog-modal"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="changelog-modal-header">
          <div className="changelog-modal-title">
            <FileText size={20} />
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

        {/* Content */}
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
              {/* Release info */}
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

              {/* Release notes */}
              <div
                className="changelog-body"
                dangerouslySetInnerHTML={{
                  __html: parseReleaseBody(release.body),
                }}
              />

              {/* Contributors section */}
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
