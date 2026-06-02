import { useState, useEffect } from 'react';
import { Download, RefreshCw, CircleCheck, CircleAlert, X, FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { relaunch } from '@tauri-apps/plugin-process';
import { isAppInApplications } from '../../hooks/useTauri';
import { formatFileSize, getErrorMessage } from '../../utils';
import { ChangelogModal } from './ChangelogModal';
import { useUpdate } from '../../contexts/UpdateContext';

type UpdateState = 'available' | 'downloading' | 'ready' | 'error';

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

export function UpdateModal() {
  const { t } = useTranslation();
  const { update, isOpen, close } = useUpdate();
  const [state, setState] = useState<UpdateState>('available');
  const [progress, setProgress] = useState<DownloadProgress>({ downloaded: 0, total: null });
  const [error, setError] = useState<string | null>(null);
  const [showChangelog, setShowChangelog] = useState(false);

  // Reset to the offer view each time the dialog reopens.
  useEffect(() => {
    if (isOpen) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Reset dialog state on open
      setState('available');
      setError(null);
      setProgress({ downloaded: 0, total: null });
    }
  }, [isOpen]);

  const handleDownloadAndInstall = async () => {
    if (!update) return;

    setState('downloading');
    setProgress({ downloaded: 0, total: null });

    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            setProgress({ downloaded: 0, total: event.data.contentLength ?? null });
            break;
          case 'Progress':
            setProgress((prev) => ({
              ...prev,
              downloaded: prev.downloaded + event.data.chunkLength,
            }));
            break;
          case 'Finished':
            // Defer 'ready' until the promise resolves; install may still fail
            break;
        }
      });

      setState('ready');
    } catch (err) {
      console.error('Failed to download update:', err);
      try {
        const inApps = await isAppInApplications();
        setError(inApps ? getErrorMessage(err, 'Download failed') : t('update.errorNotInApplications'));
      } catch {
        setError(getErrorMessage(err, 'Download failed'));
      }
      setState('error');
    }
  };

  const handleRelaunch = async () => {
    try {
      await relaunch();
    } catch (err) {
      console.error('Failed to relaunch:', err);
      setError(getErrorMessage(err, 'Failed to restart'));
      setState('error');
    }
  };

  const formatBytes = (bytes: number): string => formatFileSize(bytes, '0 B', true);

  const getProgressPercentage = (): number => {
    if (!progress.total) return 0;
    return Math.round((progress.downloaded / progress.total) * 100);
  };

  if (!isOpen || !update) return null;

  return (
    <>
      <div className="update-modal-overlay">
        <div className="update-modal">
          {state === 'available' && (
            <button className="update-modal-close" onClick={close} aria-label="Close">
              <X size={18} />
            </button>
          )}

        <div className={`update-modal-icon ${state === 'ready' ? 'success' : ''} ${state === 'error' ? 'error' : ''}`}>
          {state === 'ready' ? (
            <CircleCheck size={32} />
          ) : state === 'error' ? (
            <CircleAlert size={32} />
          ) : (
            <RefreshCw size={32} className={state === 'downloading' ? 'spinning' : ''} />
          )}
        </div>

        <h2 className="update-modal-title">
          {state === 'available' && t('update.title')}
          {state === 'downloading' && t('update.downloading')}
          {state === 'ready' && t('update.ready')}
          {state === 'error' && t('update.error')}
        </h2>

        {state === 'available' && (
          <>
            <div className="update-version-info">
              <span className="update-version-current">{update.currentVersion}</span>
              <span className="update-version-arrow">→</span>
              <span className="update-version-new">{update.version}</span>
            </div>
            <button
              className="update-changelog-link"
              onClick={() => setShowChangelog(true)}
            >
              <FileText size={14} />
              {t('update.viewChangelog', "What's New")}
            </button>
          </>
        )}

        {state === 'downloading' && (
          <div className="update-progress-container">
            <div className="update-progress-bar">
              <div
                className="update-progress-fill"
                style={{ width: `${getProgressPercentage()}%` }}
              />
            </div>
            <div className="update-progress-text">
              {progress.total ? (
                <>
                  {formatBytes(progress.downloaded)} / {formatBytes(progress.total)}
                  <span className="update-progress-percent">{getProgressPercentage()}%</span>
                </>
              ) : (
                formatBytes(progress.downloaded)
              )}
            </div>
          </div>
        )}

        {state === 'ready' && (
          <p className="update-modal-message">
            {t('update.readyMessage')}
          </p>
        )}

        {state === 'error' && (
          <p className="update-modal-message update-error-message">
            {error || t('update.errorMessage')}
          </p>
        )}

        <div className="update-modal-buttons">
          {state === 'available' && (
            <>
              <button className="update-modal-btn secondary" onClick={close}>
                {t('update.later')}
              </button>
              <button className="update-modal-btn primary" onClick={handleDownloadAndInstall}>
                <Download size={16} />
                {t('update.installNow')}
              </button>
            </>
          )}

          {state === 'downloading' && (
            <button className="update-modal-btn secondary" onClick={close}>
              {t('update.cancel')}
            </button>
          )}

          {state === 'ready' && (
            <button className="update-modal-btn primary" onClick={handleRelaunch}>
              <RefreshCw size={16} />
              {t('update.restartNow')}
            </button>
          )}

          {state === 'error' && (
            <>
              <button className="update-modal-btn secondary" onClick={close}>
                {t('update.later')}
              </button>
              <button className="update-modal-btn primary" onClick={handleDownloadAndInstall}>
                {t('update.retry')}
              </button>
            </>
          )}
        </div>
      </div>
      </div>

      {update && (
        <ChangelogModal
          isOpen={showChangelog}
          onClose={() => setShowChangelog(false)}
          version={update.version}
        />
      )}
    </>
  );
}
