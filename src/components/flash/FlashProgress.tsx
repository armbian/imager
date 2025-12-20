import { useState, useEffect, useRef, useCallback } from 'react';
import { HardDrive, Disc, FileImage } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BoardInfo, ImageInfo, BlockDevice } from '../../types';
import { getImageLogo, getOsName } from '../../assets/os-logos';
import {
  downloadImage,
  flashImage,
  getDownloadProgress,
  getFlashProgress,
  cancelOperation,
  getBoardImageUrl,
  deleteDownloadedImage,
  requestWriteAuthorization,
  checkNeedsDecompression,
  decompressCustomImage,
  getBlockDevices,
} from '../../hooks/useTauri';
import { FlashStageIcon, getStageKey, type FlashStage } from './FlashStageIcon';
import { FlashActions } from './FlashActions';
import { ErrorDisplay, MarqueeText } from '../shared';
import fallbackImage from '../../assets/armbian-logo_nofound.png';
import { POLLING } from '../../config';
import { isDeviceConnected } from '../../utils/deviceUtils';

interface FlashProgressProps {
  board: BoardInfo;
  image: ImageInfo;
  device: BlockDevice;
  onComplete: () => void;
  onBack: () => void;
}

export function FlashProgress({
  board,
  image,
  device,
  onComplete,
  onBack,
}: FlashProgressProps) {
  const { t } = useTranslation();
  const [stage, setStage] = useState<FlashStage>('authorizing');
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [boardImageUrl, setBoardImageUrl] = useState<string | null>(null);
  const [imageLoadError, setImageLoadError] = useState(false);
  const [imagePath, setImagePath] = useState<string | null>(null);
  const intervalRef = useRef<number | null>(null);
  const deviceMonitorRef = useRef<number | null>(null);
  const maxProgressRef = useRef<number>(0);
  const hasStartedRef = useRef<boolean>(false);
  const deviceDisconnectedRef = useRef<boolean>(false);

  // Cleanup downloaded image file (skip for custom images)
  async function cleanupImage(path: string | null) {
    if (image.is_custom) return;
    if (path) {
      try {
        await deleteDownloadedImage(path);
      } catch {
        // Ignore cleanup errors
      }
    }
  }

  // Handle device disconnection during flashing
  const handleDeviceDisconnected = useCallback(async () => {
    deviceDisconnectedRef.current = true;
    if (intervalRef.current) clearInterval(intervalRef.current);
    if (deviceMonitorRef.current) clearInterval(deviceMonitorRef.current);
    try {
      await cancelOperation();
    } catch {
      // Ignore
    }
    setError(t('error.deviceDisconnected'));
    setStage('error');
  }, [t]);

  // Monitor device connection during active operations
  useEffect(() => {
    const activeStages: FlashStage[] = ['downloading', 'verifying_sha', 'decompressing', 'flashing', 'verifying'];
    if (!activeStages.includes(stage)) {
      if (deviceMonitorRef.current) {
        clearInterval(deviceMonitorRef.current);
        deviceMonitorRef.current = null;
      }
      return;
    }

    const checkDevice = async () => {
      try {
        const devices = await getBlockDevices();
        if (!isDeviceConnected(device.path, devices)) {
          handleDeviceDisconnected();
        }
      } catch {
        // Ignore polling errors
      }
    };

    checkDevice();
    deviceMonitorRef.current = window.setInterval(checkDevice, POLLING.DEVICE_CHECK);

    return () => {
      if (deviceMonitorRef.current) {
        clearInterval(deviceMonitorRef.current);
        deviceMonitorRef.current = null;
      }
    };
  }, [stage, device.path, handleDeviceDisconnected]);

  async function loadBoardImage() {
    try {
      const url = await getBoardImageUrl(board.slug);
      setBoardImageUrl(url);
    } catch {
      // Ignore
    }
  }

  async function handleAuthorization() {
    setStage('authorizing');
    setProgress(0);
    setError(null);

    try {
      // On Linux, if not root, this will trigger pkexec and restart the app
      // The app will restart elevated and the user will need to re-select options
      const authorized = await requestWriteAuthorization(device.path);
      if (!authorized) {
        setError(t('error.authCancelled'));
        setStage('error');
        return;
      }

      if (image.is_custom && image.custom_path) {
        await handleCustomImage(image.custom_path);
      } else {
        startDownload();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : t('error.authFailed'));
      setStage('error');
    }
  }

  useEffect(() => {
    if (hasStartedRef.current) return;
    hasStartedRef.current = true;

    loadBoardImage();
    handleAuthorization();

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleCustomImage(customPath: string) {
    try {
      const needsDecompress = await checkNeedsDecompression(customPath);

      if (needsDecompress) {
        setStage('decompressing');
        setProgress(0);
        const decompressedPath = await decompressCustomImage(customPath);
        setImagePath(decompressedPath);
        startFlash(decompressedPath);
      } else {
        setImagePath(customPath);
        startFlash(customPath);
      }
    } catch (err) {
      if (deviceDisconnectedRef.current) return;

      // Check if device is still connected before showing decompression error
      try {
        const devices = await getBlockDevices();
        if (!isDeviceConnected(device.path, devices)) {
          handleDeviceDisconnected();
          return;
        }
      } catch {
        // If we can't check, continue with decompression error
      }

      setError(err instanceof Error ? err.message : t('error.decompressionFailed'));
      setStage('error');
    }
  }

  async function startDownload() {
    setStage('downloading');
    setProgress(0);
    setError(null);
    maxProgressRef.current = 0;

    intervalRef.current = window.setInterval(async () => {
      try {
        const prog = await getDownloadProgress();

        if (prog.is_verifying_sha && stage !== 'verifying_sha') {
          setStage('verifying_sha');
          maxProgressRef.current = 0;
          setProgress(0);
        } else if (prog.is_decompressing && stage !== 'decompressing') {
          setStage('decompressing');
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

        if (prog.error && !deviceDisconnectedRef.current) {
          setError(prog.error);
          setStage('error');
          if (intervalRef.current) clearInterval(intervalRef.current);
        }
      } catch {
        // Ignore polling errors
      }
    }, POLLING.DOWNLOAD_PROGRESS);

    try {
      const path = await downloadImage(image.file_url, image.file_url_sha);
      setImagePath(path);
      if (intervalRef.current) clearInterval(intervalRef.current);
      startFlash(path);
    } catch (err) {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (deviceDisconnectedRef.current) return;

      // Check if device is still connected before showing download error
      try {
        const devices = await getBlockDevices();
        if (!isDeviceConnected(device.path, devices)) {
          handleDeviceDisconnected();
          return;
        }
      } catch {
        // If we can't check, continue with download error
      }

      setError(err instanceof Error ? err.message : t('error.downloadFailed'));
      setStage('error');
    }
  }

  async function startFlash(path: string) {
    setStage('flashing');
    setProgress(0);
    maxProgressRef.current = 0;

    intervalRef.current = window.setInterval(async () => {
      try {
        const prog = await getFlashProgress();
        if (prog.is_verifying) {
          setStage('verifying');
          if (maxProgressRef.current > 50) {
            maxProgressRef.current = 0;
          }
        }
        if (prog.progress_percent >= maxProgressRef.current) {
          maxProgressRef.current = prog.progress_percent;
          setProgress(prog.progress_percent);
        }
        if (prog.error && !deviceDisconnectedRef.current) {
          setError(prog.error);
          setStage('error');
          if (intervalRef.current) clearInterval(intervalRef.current);
        }
      } catch {
        // Ignore polling errors
      }
    }, POLLING.FLASH_PROGRESS);

    try {
      await flashImage(path, device.path, true);
      if (intervalRef.current) clearInterval(intervalRef.current);
      setStage('complete');
      setProgress(100);
    } catch (err) {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (deviceDisconnectedRef.current) return;

      // Check if device is still connected before showing flash error
      try {
        const devices = await getBlockDevices();
        if (!isDeviceConnected(device.path, devices)) {
          handleDeviceDisconnected();
          return;
        }
      } catch {
        // If we can't check, assume disconnected on certain errors
      }

      setError(err instanceof Error ? err.message : t('error.flashFailed'));
      setStage('error');
    }
  }

  async function handleCancel() {
    try {
      await cancelOperation();
      if (intervalRef.current) clearInterval(intervalRef.current);
      await cleanupImage(imagePath);
      onBack();
    } catch {
      // Ignore
    }
  }

  async function handleRetry() {
    setError(null);
    deviceDisconnectedRef.current = false;

    // If device was disconnected, need to re-authorize
    if (imagePath) {
      // Re-authorize before flashing
      setStage('authorizing');
      try {
        const authorized = await requestWriteAuthorization(device.path);
        if (!authorized) {
          setError(t('error.authCancelled'));
          setStage('error');
          return;
        }
        startFlash(imagePath);
      } catch (err) {
        setError(err instanceof Error ? err.message : t('error.authFailed'));
        setStage('error');
      }
    } else if (image.is_custom && image.custom_path) {
      handleAuthorization();
    } else {
      handleAuthorization();
    }
  }

  async function handleBack() {
    await cleanupImage(imagePath);
    onBack();
  }

  function getImageDisplayText(): string {
    if (image.is_custom) {
      return image.distro_release;
    }
    return `Armbian ${image.armbian_version} ${image.distro_release}`;
  }

  const showHeader = stage !== 'authorizing' && stage !== 'error';

  return (
    <div className={`flash-container ${!showHeader ? 'centered' : ''}`}>
      {showHeader && (
        <div className="flash-header">
          {image.is_custom ? (
            <div className="flash-board-image flash-custom-image-icon">
              <FileImage size={40} />
            </div>
          ) : (
            <img
              src={imageLoadError ? fallbackImage : (boardImageUrl || fallbackImage)}
              alt={board.name}
              className="flash-board-image"
              onError={() => setImageLoadError(true)}
            />
          )}
          <div className="flash-info">
            <h2>{board.name}</h2>
            <div className="flash-info-badges">
              <div
                className="os-badge"
                title={image.is_custom ? image.distro_release : undefined}
              >
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
                <HardDrive size={16} />
                <MarqueeText text={device.model || device.name} maxWidth={150} className="flash-device-name" />
                <span className="flash-device-size">{device.size_formatted}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      <div className={`flash-status ${stage}`}>
        <FlashStageIcon stage={stage} />
        <h3>{t(getStageKey(stage))}</h3>

        {stage !== 'complete' &&
          stage !== 'error' &&
          stage !== 'authorizing' && (
            <div className="progress-container">
              <div
                className={`progress-bar ${stage === 'decompressing' || stage === 'verifying_sha' ? 'indeterminate' : ''}`}
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

        {stage === 'complete' && (
          <p className="flash-success-hint">
            {image.is_custom
              ? t('flash.successHintCustom')
              : t('flash.successHint', { boardName: board.name })}
          </p>
        )}

        {error && <ErrorDisplay error={error} />}

        <FlashActions
          stage={stage}
          onComplete={onComplete}
          onBack={handleBack}
          onRetry={handleRetry}
          onCancel={handleCancel}
        />
      </div>
    </div>
  );
}
