import { useState, useEffect, useCallback, useRef } from 'react';
import { Download, RefreshCw, CheckCircle, AlertCircle, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { check, Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { logInfo } from '../../hooks/useTauri';

type UpdateState = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error';

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

export function UpdateModal() {
  const { t } = useTranslation();
  const [state, setState] = useState<UpdateState>('idle');
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<DownloadProgress>({ downloaded: 0, total: null });
  const [error, setError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const hasCheckedRef = useRef(false);

  const checkForUpdate = useCallback(async () => {
    if (hasCheckedRef.current) return;
    hasCheckedRef.current = true;

    setState('checking');
    setError(null);

    try {
      const updateResult = await check();

      if (updateResult) {
        setUpdate(updateResult);
        setState('available');
        logInfo('updater', `Update available: ${updateResult.currentVersion} -> ${updateResult.version}`);
      } else {
        logInfo('updater', 'No updates available');
        setState('idle');
      }
    } catch (err) {
      console.error('Failed to check for updates:', err);
      // Silently fail - don't show error modal for update check failures
      setState('idle');
    }
  }, []);

  useEffect(() => {
    checkForUpdate();
  }, [checkForUpdate]);

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
            setState('ready');
            break;
        }
      });

      setState('ready');
    } catch (err) {
      console.error('Failed to download update:', err);
      setError(err instanceof Error ? err.message : 'Download failed');
      setState('error');
    }
  };

  const handleRelaunch = async () => {
    try {
      await relaunch();
    } catch (err) {
      console.error('Failed to relaunch:', err);
      setError(err instanceof Error ? err.message : 'Failed to restart');
      setState('error');
    }
  };

  const handleLater = () => {
    setDismissed(true);
  };

  const handleRetry = () => {
    if (state === 'error' && update) {
      handleDownloadAndInstall();
    } else {
      checkForUpdate();
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getProgressPercentage = (): number => {
    if (!progress.total) return 0;
    return Math.round((progress.downloaded / progress.total) * 100);
  };

  // Don't show anything if no update or dismissed
  if (state === 'idle' || state === 'checking' || dismissed) return null;

  return (
    <div className="update-modal-overlay">
      <div className="update-modal">
        {/* Close button for available state */}
        {state === 'available' && (
          <button className="update-modal-close" onClick={handleLater} aria-label="Close">
            <X size={18} />
          </button>
        )}

        {/* Icon */}
        <div className={`update-modal-icon ${state === 'ready' ? 'success' : ''} ${state === 'error' ? 'error' : ''}`}>
          {state === 'ready' ? (
            <CheckCircle size={32} />
          ) : state === 'error' ? (
            <AlertCircle size={32} />
          ) : (
            <RefreshCw size={32} className={state === 'downloading' ? 'spinning' : ''} />
          )}
        </div>

        {/* Title */}
        <h2 className="update-modal-title">
          {state === 'available' && t('update.title')}
          {state === 'downloading' && t('update.downloading')}
          {state === 'ready' && t('update.ready')}
          {state === 'error' && t('update.error')}
        </h2>

        {/* Message / Content */}
        {state === 'available' && update && (
          <div className="update-version-info">
            <span className="update-version-current">{update.currentVersion}</span>
            <span className="update-version-arrow">→</span>
            <span className="update-version-new">{update.version}</span>
          </div>
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

        {/* Buttons */}
        <div className="update-modal-buttons">
          {state === 'available' && (
            <>
              <button className="update-modal-btn secondary" onClick={handleLater}>
                {t('update.later')}
              </button>
              <button className="update-modal-btn primary" onClick={handleDownloadAndInstall}>
                <Download size={16} />
                {t('update.installNow')}
              </button>
            </>
          )}

          {state === 'downloading' && (
            <button className="update-modal-btn secondary" onClick={handleLater}>
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
              <button className="update-modal-btn secondary" onClick={handleLater}>
                {t('update.later')}
              </button>
              <button className="update-modal-btn primary" onClick={handleRetry}>
                {t('update.retry')}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
