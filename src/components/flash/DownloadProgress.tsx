import { useState, useEffect, useRef } from 'react';
import { Disc, FolderOpen } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BoardInfo, ImageInfo } from '../../types';
import { getImageLogo, getOsName } from '../../assets/os-logos';
import {
  selectSavePath,
  downloadToPath,
  getDownloadProgress,
  cancelOperation,
  getBoardImageUrl,
} from '../../hooks/useTauri';
import { ErrorDisplay, MarqueeText } from '../shared';
import fallbackImage from '../../assets/armbian-logo_nofound.png';
import { POLLING } from '../../config';
import {
  Download,
  Archive,
  CheckCircle,
  XCircle,
  ShieldCheck,
  FolderOpen as FolderIcon,
} from 'lucide-react';

type DownloadStage =
  | 'selecting'
  | 'downloading'
  | 'verifying_sha'
  | 'decompressing'
  | 'complete'
  | 'error'
  | 'cancelled';

interface DownloadProgressProps {
  board: BoardInfo;
  image: ImageInfo;
  decompress: boolean;
  onComplete: () => void;
  onBack: () => void;
}

function DownloadStageIcon({ stage, size = 48 }: { stage: DownloadStage; size?: number }) {
  switch (stage) {
    case 'selecting':
      return <FolderIcon size={size} className="stage-icon selecting" />;
    case 'downloading':
      return <Download size={size} className="stage-icon downloading" />;
    case 'verifying_sha':
      return <ShieldCheck size={size} className="stage-icon verifying-sha" />;
    case 'decompressing':
      return <Archive size={size} className="stage-icon decompressing" />;
    case 'complete':
      return <CheckCircle size={size} className="stage-icon complete" />;
    case 'error':
    case 'cancelled':
      return <XCircle size={size} className="stage-icon error" />;
  }
}

function getDownloadStageKey(stage: DownloadStage): string {
  switch (stage) {
    case 'selecting':
      return 'download.selecting';
    case 'downloading':
      return 'download.downloading';
    case 'verifying_sha':
      return 'download.verifyingSha';
    case 'decompressing':
      return 'download.decompressing';
    case 'complete':
      return 'download.complete';
    case 'cancelled':
      return 'download.cancelled';
    case 'error':
      return 'download.failed';
  }
}

export function DownloadProgress({
  board,
  image,
  decompress,
  onComplete,
  onBack,
}: DownloadProgressProps) {
  const { t } = useTranslation();
  const [stage, setStage] = useState<DownloadStage>('selecting');
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [boardImageUrl, setBoardImageUrl] = useState<string | null>(null);
  const [imageLoadError, setImageLoadError] = useState(false);
  const intervalRef = useRef<number | null>(null);
  const maxProgressRef = useRef<number>(0);
  const hasStartedRef = useRef<boolean>(false);
  const stageRef = useRef<DownloadStage>('selecting');

  // Extract filename from URL
  function getFilenameFromUrl(url: string): string {
    const urlPath = url.split('?')[0];
    const parts = urlPath.split('/');
    return parts[parts.length - 1] || 'image.img';
  }

  async function loadBoardImage() {
    try {
      const url = await getBoardImageUrl(board.slug);
      setBoardImageUrl(url);
    } catch {
      // Ignore
    }
  }

  // Helper to update stage and ref together
  function updateStage(newStage: DownloadStage) {
    stageRef.current = newStage;
    setStage(newStage);
  }

  // Start polling for download progress
  function startPolling() {
    intervalRef.current = window.setInterval(async () => {
      try {
        const prog = await getDownloadProgress();

        // Use ref to check current stage (avoids stale closure)
        if (prog.is_verifying_sha && stageRef.current !== 'verifying_sha') {
          updateStage('verifying_sha');
          maxProgressRef.current = 0;
          setProgress(0);
        } else if (prog.is_decompressing && stageRef.current !== 'decompressing') {
          updateStage('decompressing');
          maxProgressRef.current = 0;
          setProgress(0);
        }

        if (!prog.is_decompressing && !prog.is_verifying_sha) {
          const newProgress = prog.progress_percent;
          if (newProgress >= maxProgressRef.current) {
            maxProgressRef.current = newProgress;
            setProgress(newProgress);
          }
        }

        if (prog.error) {
          setError(prog.error);
          updateStage('error');
          if (intervalRef.current) clearInterval(intervalRef.current);
        }
      } catch {
        // Ignore polling errors
      }
    }, POLLING.DOWNLOAD_PROGRESS);
  }

  async function startDownload() {
    setProgress(0);
    setError(null);
    maxProgressRef.current = 0;

    try {
      // Step 1: Show save dialog and get path (no download yet)
      const suggestedFilename = getFilenameFromUrl(image.file_url);
      const savePath = await selectSavePath(suggestedFilename, decompress);
      
      if (!savePath) {
        // User cancelled the save dialog
        updateStage('cancelled');
        setError(t('download.cancelled'));
        return;
      }

      // Step 2: Now start polling (download is about to begin)
      updateStage('downloading');
      startPolling();

      // Step 3: Start the actual download
      const path = await downloadToPath(
        image.file_url,
        image.file_url_sha,
        savePath,
        decompress
      );
      
      if (intervalRef.current) clearInterval(intervalRef.current);
      setSavedPath(path);
      updateStage('complete');
      setProgress(100);
    } catch (err) {
      if (intervalRef.current) clearInterval(intervalRef.current);
      
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      updateStage('error');
    }
  }

  useEffect(() => {
    if (hasStartedRef.current) return;
    hasStartedRef.current = true;

    loadBoardImage();
    startDownload();

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleCancel() {
    try {
      await cancelOperation();
      if (intervalRef.current) clearInterval(intervalRef.current);
      onBack();
    } catch {
      // Ignore
    }
  }

  async function handleRetry() {
    setError(null);
    hasStartedRef.current = false;
    startDownload();
  }

  function getImageDisplayText(): string {
    return `Armbian ${image.armbian_version} ${image.distro_release}`;
  }

  const showHeader = stage !== 'error' && stage !== 'cancelled';

  return (
    <div className={`flash-container ${!showHeader ? 'centered' : ''}`}>
      {showHeader && (
        <div className="flash-header">
          <img
            src={imageLoadError ? fallbackImage : (boardImageUrl || fallbackImage)}
            alt={board.name}
            className="flash-board-image"
            onError={() => setImageLoadError(true)}
          />
          <div className="flash-info">
            <h2>{board.name}</h2>
            <div className="flash-info-badges">
              <div className="os-badge">
                {(() => {
                  const logo = getImageLogo(
                    image.distro_release,
                    image.preinstalled_application
                  );
                  return logo ? (
                    <img
                      src={logo}
                      alt={getOsName(image.distro_release)}
                      className="os-badge-logo"
                    />
                  ) : (
                    <Disc size={20} className="os-badge-icon" />
                  );
                })()}
                <MarqueeText text={getImageDisplayText()} maxWidth={200} className="os-badge-text" />
              </div>
              <div className="flash-device-row">
                <FolderOpen size={16} />
                <span className="flash-device-name">{t('download.savingToDisk')}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      <div className={`flash-status ${stage}`}>
        <DownloadStageIcon stage={stage} />
        <h3>{t(getDownloadStageKey(stage))}</h3>

        {stage !== 'complete' &&
          stage !== 'error' &&
          stage !== 'cancelled' &&
          stage !== 'selecting' && (
            <div className="progress-container">
              <div
                className={`progress-bar ${
                  stage === 'decompressing' || stage === 'verifying_sha' ? 'indeterminate' : ''
                }`}
              >
                <div
                  className="progress-fill"
                  style={{
                    width: stage === 'decompressing' || stage === 'verifying_sha' ? '100%' : `${progress}%`,
                  }}
                />
              </div>
              {stage !== 'decompressing' && stage !== 'verifying_sha' && (
                <span className="progress-text">{progress.toFixed(0)}%</span>
              )}
            </div>
          )}

        {stage === 'complete' && savedPath && (
          <p className="flash-success-hint">
            {t('download.successHint', { path: savedPath })}
          </p>
        )}

        {error && <ErrorDisplay error={error} />}

        <div className="flash-actions-inline">
          {stage === 'complete' && (
            <>
              <button className="btn btn-secondary" onClick={onBack}>
                {t('download.downloadAnother')}
              </button>
              <button className="btn btn-primary" onClick={onComplete}>
                {t('flash.done')}
              </button>
            </>
          )}
          {(stage === 'error' || stage === 'cancelled') && (
            <>
              <button className="btn btn-secondary" onClick={onBack}>
                {t('flash.cancel')}
              </button>
              {stage === 'error' && (
                <button className="btn btn-primary" onClick={handleRetry}>
                  {t('flash.retry')}
                </button>
              )}
            </>
          )}
          {stage !== 'complete' && stage !== 'error' && stage !== 'cancelled' && stage !== 'selecting' && (
            <button className="btn btn-secondary" onClick={handleCancel}>
              {t('flash.cancel')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
