import { useState, useEffect, useCallback, useRef } from 'react';
import { Download, RefreshCw, CheckCircle, AlertCircle, X, FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { check, Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { logInfo, isAppInApplications } from '../../hooks/useTauri';
import { formatFileSize, getErrorMessage } from '../../utils';
import { ChangelogModal } from './ChangelogModal';
import { getShowUpdaterModal } from '../../hooks/useSettings';

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
  const [showChangelog, setShowChangelog] = useState(false);
  const hasCheckedRef = useRef(false);
  const hasLoggedDisabledRef = useRef(false);

  const checkForUpdate = useCallback(async () => {
    if (hasCheckedRef.current) return;
    hasCheckedRef.current = true;

    // Check if updater modal is enabled in settings
    const showModal = await getShowUpdaterModal();
    if (!showModal) {
      // Log this message only once per session
      if (!hasLoggedDisabledRef.current) {
        logInfo('updater', 'Updater modal disabled in settings, skipping update check');
        hasLoggedDisabledRef.current = true;
      }
      setState('idle');
      return;
    }

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
      // Update check failures are non-critical - user can continue using current version
      setState('idle');
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- Initial update check on mount
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
            // Download complete - don't set 'ready' here since install may still fail.
            // State is set to 'ready' after downloadAndInstall() promise resolves.
            break;
        }
      });

      setState('ready');
    } catch (err) {
      console.error('Failed to download update:', err);
      try {
        const inApps = await isAppInApplications();
        if (!inApps) {
          setError(t('update.errorNotInApplications'));
        } else {
          setError(getErrorMessage(err, 'Download failed'));
        }
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

  /** Format file size in human-readable format with decimal precision for small values */
  const formatBytes = (bytes: number): string => formatFileSize(bytes, '0 B', true);

  const getProgressPercentage = (): number => {
    if (!progress.total) return 0;
    return Math.round((progress.downloaded / progress.total) * 100);
  };

  // Hide modal when no update is available or user has dismissed it
  if (state === 'idle' || state === 'checking' || dismissed) return null;

  return (
    <>
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

      {/* Changelog Modal */}
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
