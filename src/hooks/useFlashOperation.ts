/**
 * Hook that encapsulates the entire flash operation lifecycle:
 * authorization, download, decompression, flashing, verification,
 * device monitoring, progress polling, error recovery, and cleanup.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { ImageInfo, BlockDevice } from '../types';
import type { FlashStage } from '../components/flash/FlashStageIcon';
import {
  downloadImage,
  flashImage,
  getDownloadProgress,
  getFlashProgress,
  cancelOperation,
  deleteDownloadedImage,
  deleteDecompressedCustomImage,
  forceDeleteCachedImage,
  requestWriteAuthorization,
  checkNeedsDecompression,
  decompressCustomImage,
  getBlockDevices,
  continueDownloadWithoutSha,
  cleanupFailedDownload,
} from './useTauri';
import { getSkipVerify } from './useSettings';
import { POLLING, CACHE, STORAGE_KEYS } from '../config';
import { isDeviceConnected } from '../utils/deviceUtils';

interface UseFlashOperationProps {
  image: ImageInfo;
  device: BlockDevice;
  onBack: () => void;
}

interface UseFlashOperationReturn {
  stage: FlashStage;
  progress: number;
  error: string | null;
  imagePath: string | null;
  showShaWarning: boolean;
  handleCancel: () => Promise<void>;
  handleRetry: () => Promise<void>;
  handleBack: () => Promise<void>;
  handleShaWarningConfirm: () => Promise<void>;
  handleShaWarningCancel: () => Promise<void>;
}

/**
 * Safely cleanup an image file, ignoring errors
 * Handles both custom (decompressed) and downloaded images
 */
async function cleanupImageSafely(
  path: string | null,
  isCustom?: boolean
): Promise<void> {
  if (!path) return;
  try {
    if (isCustom) {
      await deleteDecompressedCustomImage(path);
    } else {
      await deleteDownloadedImage(path);
    }
  } catch {
    // Ignore cleanup errors
  }
}

/**
 * Check if a SHA error indicates the SHA file was unavailable (not a mismatch)
 */
function isShaUnavailableError(error: string): boolean {
  return error.includes('[SHA_UNAVAILABLE]');
}

export function useFlashOperation({
  image,
  device,
  onBack,
}: UseFlashOperationProps): UseFlashOperationReturn {
  const { t } = useTranslation();

  // Operation state
  const [stage, setStage] = useState<FlashStage>('authorizing');
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [showShaWarning, setShowShaWarning] = useState(false);

  // Refs for lifecycle management
  const intervalRef = useRef<number | null>(null);
  const deviceMonitorRef = useRef<number | null>(null);
  const maxProgressRef = useRef<number>(0);
  const hasStartedRef = useRef<boolean>(false);
  const deviceDisconnectedRef = useRef<boolean>(false);
  const skipVerifyRef = useRef<boolean>(false);

  // Failure tracking via sessionStorage
  const failureStorageKey = `${STORAGE_KEYS.FLASH_FAILURE_PREFIX}${image.file_url}`;

  const getFlashFailureCount = (): number => {
    try {
      const stored = sessionStorage.getItem(failureStorageKey);
      return stored ? parseInt(stored, 10) : 0;
    } catch {
      return 0;
    }
  };

  const setFlashFailureCount = (count: number): void => {
    try {
      if (count === 0) {
        sessionStorage.removeItem(failureStorageKey);
      } else {
        sessionStorage.setItem(failureStorageKey, count.toString());
      }
    } catch {
      // Ignore storage errors
    }
  };

  /** Clear all active polling intervals */
  const clearIntervals = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    if (deviceMonitorRef.current) {
      clearInterval(deviceMonitorRef.current);
      deviceMonitorRef.current = null;
    }
  }, []);

  /**
   * Check device connection and trigger disconnect handler if missing
   * Returns true if device is still connected
   */
  const checkDeviceOrDisconnect = useCallback(async (): Promise<boolean> => {
    try {
      const devices = await getBlockDevices();
      if (!isDeviceConnected(device.path, devices)) {
        await handleDeviceDisconnectedInternal();
        return false;
      }
    } catch {
      // If we can't check, assume still connected
    }
    return true;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.path]);

  /** Handle device disconnection during operations */
  const handleDeviceDisconnectedInternal = useCallback(async () => {
    deviceDisconnectedRef.current = true;
    setShowShaWarning(false);
    try {
      await cleanupFailedDownload();
    } catch {
      // Ignore cleanup errors
    }
    clearIntervals();
    try {
      await cancelOperation();
    } catch {
      // Ignore
    }
    setError(t('error.deviceDisconnected'));
    setStage('error');
  }, [t, clearIntervals]);

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
          handleDeviceDisconnectedInternal();
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
  }, [stage, device.path, handleDeviceDisconnectedInternal]);

  /** Handle custom image flow (decompress if needed, then flash) */
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
      if (!(await checkDeviceOrDisconnect())) return;

      setError(err instanceof Error ? err.message : t('error.decompressionFailed'));
      setStage('error');
    }
  }

  /** Start download with progress polling */
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

      const errorMsg = err instanceof Error ? err.message : String(err);

      // SHA fetch failed - show modal (file already downloaded, not deleted)
      if (isShaUnavailableError(errorMsg)) {
        setShowShaWarning(true);
        return;
      }

      if (!(await checkDeviceOrDisconnect())) return;

      setError(err instanceof Error ? err.message : t('error.downloadFailed'));
      setStage('error');
    }
  }

  /** Start flash with progress polling */
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
      await flashImage(path, device.path, !skipVerifyRef.current);
      if (intervalRef.current) clearInterval(intervalRef.current);
      setStage('complete');
      setProgress(100);
      setFlashFailureCount(0);
      await cleanupImageSafely(path, image.is_custom);
    } catch (err) {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (deviceDisconnectedRef.current) return;

      if (!(await checkDeviceOrDisconnect())) return;

      // Increment failure count for cached (non-custom) images
      if (!image.is_custom) {
        const currentCount = getFlashFailureCount() + 1;
        setFlashFailureCount(currentCount);

        // Auto-drop cached image after too many failures (possibly corrupted)
        if (currentCount >= CACHE.MAX_FLASH_FAILURES) {
          try {
            await forceDeleteCachedImage(path);
            setFlashFailureCount(0);
          } catch {
            // Ignore deletion errors
          }
        }
      }

      await cleanupImageSafely(path, image.is_custom);
      setError(err instanceof Error ? err.message : t('error.flashFailed'));
      setStage('error');
    }
  }

  /** Authorization flow - entry point for the operation */
  async function handleAuthorization() {
    setStage('authorizing');
    setProgress(0);
    setError(null);

    try {
      // Load skip verify setting once at the start
      try {
        skipVerifyRef.current = await getSkipVerify();
      } catch {
        skipVerifyRef.current = false;
      }

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

  // Start operation on mount (once)
  useEffect(() => {
    if (hasStartedRef.current) return;
    hasStartedRef.current = true;

    handleAuthorization();

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // === Public action handlers ===

  const handleCancel = async () => {
    try {
      await cancelOperation();
      if (intervalRef.current) clearInterval(intervalRef.current);
      await cleanupImageSafely(imagePath, image.is_custom);
      onBack();
    } catch {
      // Ignore
    }
  };

  const handleRetry = async () => {
    setError(null);
    deviceDisconnectedRef.current = false;

    if (imagePath) {
      // Re-authorize before re-flashing existing image
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
    } else {
      handleAuthorization();
    }
  };

  const handleBack = async () => {
    await cleanupImageSafely(imagePath, image.is_custom);
    onBack();
  };

  const handleShaWarningConfirm = async () => {
    setShowShaWarning(false);

    if (!(await checkDeviceOrDisconnect())) return;

    setStage('decompressing');
    setProgress(0);

    try {
      const path = await continueDownloadWithoutSha();
      setImagePath(path);
      startFlash(path);
    } catch (err) {
      if (deviceDisconnectedRef.current) return;
      setError(err instanceof Error ? err.message : t('error.decompressionFailed'));
      setStage('error');
    }
  };

  const handleShaWarningCancel = async () => {
    setShowShaWarning(false);
    await cleanupFailedDownload();
    onBack();
  };

  return {
    stage,
    progress,
    error,
    imagePath,
    showShaWarning,
    handleCancel,
    handleRetry,
    handleBack,
    handleShaWarningConfirm,
    handleShaWarningCancel,
  };
}
