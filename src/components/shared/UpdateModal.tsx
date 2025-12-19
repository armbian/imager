import { useState, useEffect } from 'react';
import { Download, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '../../hooks/useTauri';

const GITHUB_API_URL = 'https://api.github.com/repos/armbian/imager/releases/latest';
const RELEASES_URL = 'https://github.com/armbian/imager/releases/latest';

interface GitHubRelease {
  tag_name: string;
  html_url: string;
}

function compareVersions(current: string, latest: string): number {
  const c = current.replace(/^v/, '').split('.').map(Number);
  const l = latest.replace(/^v/, '').split('.').map(Number);

  for (let i = 0; i < Math.max(c.length, l.length); i++) {
    const cv = c[i] || 0;
    const lv = l[i] || 0;
    if (cv < lv) return -1;
    if (cv > lv) return 1;
  }
  return 0;
}

export function UpdateModal() {
  const { t } = useTranslation();
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [latestVersion, setLatestVersion] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    const checkForUpdate = async () => {
      try {
        const currentVersion = await getVersion();
        const response = await fetch(GITHUB_API_URL);

        if (!response.ok) return;

        const release: GitHubRelease = await response.json();
        const latest = release.tag_name;

        if (compareVersions(currentVersion, latest) < 0) {
          setLatestVersion(latest);
          setUpdateAvailable(true);
        }
      } catch (err) {
        console.error('Failed to check for updates:', err);
      }
    };

    checkForUpdate();
  }, []);

  const handleDownload = () => {
    openUrl(RELEASES_URL).catch(console.error);
    setDismissed(true);
  };

  const handleLater = () => {
    setDismissed(true);
  };

  if (!updateAvailable || dismissed) return null;

  return (
    <div className="update-modal-overlay">
      <div className="update-modal">
        <div className="update-modal-icon">
          <RefreshCw size={32} />
        </div>
        <h2 className="update-modal-title">{t('update.title')}</h2>
        <p className="update-modal-message">
          {t('update.newVersionAvailable', { version: latestVersion })}
        </p>
        <div className="update-modal-buttons">
          <button className="update-modal-btn secondary" onClick={handleLater}>
            {t('update.later')}
          </button>
          <button className="update-modal-btn primary" onClick={handleDownload}>
            <Download size={16} />
            {t('update.download')}
          </button>
        </div>
      </div>
    </div>
  );
}
