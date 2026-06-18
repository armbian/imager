// Drives the full flash lifecycle: authorize, download, decompress, flash, verify, cleanup

import { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { ImageInfo, BlockDevice, AutoconfigConfig } from '../types';
import { FLASH_METHOD, deriveFlashMethod, isEdlMethod } from '../types';
import { PHASE_ORDER, type FlashStage, type FlashPhase } from '../components/flash/FlashStageIcon';
import {
  downloadImage,
  flashImage,
  flashQdlImage,
  flashQdlUfsImage,
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
  getQdlDevices,
  continueDownloadWithoutSha,
  cleanupFailedDownload,
  listCachedImages,
} from './useTauri';
import { getSkipVerify } from './useSettings';
import { POLLING, CACHE, STORAGE_KEYS } from '../config';
import { getErrorMessage, armbianIdentityKey, isCompressedImage } from '../utils';
import { isDeviceConnected } from '../utils/deviceUtils';
import { isShaUnavailableError, translateFlashError } from '../utils/errorUtils';

interface UseFlashOperationProps {
  image: ImageInfo;
  device: BlockDevice;
  /** Board SoC, used to select the firehose loader for UFS (QDL) flashing. */
  soc?: string;
  /** Board slug, the fallback loader key when the API leaves `soc` null. */
  boardSlug?: string;
  /** Opt-in autoconfig profile config written into the image on first boot; null when none. */
  autoconfig?: AutoconfigConfig | null;
  onBack: () => void;
}

interface UseFlashOperationReturn {
  stage: FlashStage;
  phases: FlashPhase[];
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

/** Delete a custom (decompressed) or downloaded image file, ignoring errors */
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


function buildPhases(opts: { download: boolean; prepare: boolean; verify: boolean }): FlashPhase[] {
  const phases: FlashPhase[] = [];
  if (opts.download) phases.push('download');
  if (opts.prepare) phases.push('prepare');
  phases.push('write');
  if (opts.verify) phases.push('verify');
  return phases;
}

export function useFlashOperation({
  image,
  device,
  soc,
  boardSlug,
  autoconfig,
  onBack,
}: UseFlashOperationProps): UseFlashOperationReturn {
  const { t } = useTranslation();

  // Operation state
  const [stage, setStage] = useState<FlashStage>('authorizing');
  const [phases, setPhases] = useState<FlashPhase[]>(() => [...PHASE_ORDER]);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [showShaWarning, setShowShaWarning] = useState(false);

  // Refs for lifecycle management
  const intervalRef = useRef<number | null>(null);
  const deviceMonitorRef = useRef<number | null>(null);
  const maxProgressRef = useRef<number>(0);
  // True once this flash's write phase is observed; guards against a stale is_verifying from a
  // previous run latching the UI onto "verifying" with a full bar before this run writes.
  const flashWriteSeenRef = useRef<boolean>(false);
  const hasStartedRef = useRef<boolean>(false);
  const deviceDisconnectedRef = useRef<boolean>(false);
  const userCancelledRef = useRef<boolean>(false);
  // Precedence latch: a specific error (real write failure) beats the generic
  // "device disconnected", never the reverse, regardless of which path fires first.
  const shownErrorRef = useRef<null | 'generic' | 'specific'>(null);
  const pendingCleanupRef = useRef<Promise<void> | null>(null);
  const skipVerifyRef = useRef<boolean>(false);
  // Keep the latest opt-in profile config for the event-driven flash flow.
  const autoconfigRef = useRef<AutoconfigConfig | null>(autoconfig ?? null);
  autoconfigRef.current = autoconfig ?? null;

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

  /** Single exit into the error screen: never empty, honors the precedence latch. */
  const failFlash = useCallback(
    (message: string, specific = true) => {
      if (shownErrorRef.current === 'specific') return;
      if (shownErrorRef.current === 'generic' && !specific) return;
      shownErrorRef.current = specific ? 'specific' : 'generic';
      setError(message.trim() || t('error.flashFailed'));
      setStage('error');
    },
    [t]
  );

  // Write path: TAR-based QDL ('qdl'), raw UFS over QDL ('qdl-ufs'), or block dd.
  const flashMethod = deriveFlashMethod(image);
  // isQdlMode == the TAR path specifically (extract + rawprogram, no download cleanup).
  const isQdlMode = flashMethod === FLASH_METHOD.QDL;
  const isUfsMode = flashMethod === FLASH_METHOD.QDL_UFS;
  // Both flash an EDL device, so they share device handling, auth skip, and no block verify.
  const isEdlFlash = isEdlMethod(flashMethod);

  /** Check the device is connected; trigger the disconnect handler and return false if not */
  const checkDeviceOrDisconnect = useCallback(async (): Promise<boolean> => {
    try {
      if (isEdlFlash) {
        const qdlDevices = await getQdlDevices();
        if (qdlDevices.length === 0) {
          await handleDeviceDisconnectedInternal();
          return false;
        }
      } else {
        const devices = await getBlockDevices();
        if (!isDeviceConnected(device.path, devices)) {
          await handleDeviceDisconnectedInternal();
          return false;
        }
      }
    } catch {
      // If we can't check, assume still connected
    }
    return true;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.path, isEdlFlash]);

  /** Handle device disconnection during operations */
  const handleDeviceDisconnectedInternal = useCallback(async () => {
    if (deviceDisconnectedRef.current) return;
    deviceDisconnectedRef.current = true;
    setShowShaWarning(false);
    clearIntervals();
    // Error state first, synchronously: the UI must never wait on backend cleanup.
    failFlash(t('error.deviceDisconnected'), false);
    pendingCleanupRef.current = (async () => {
      try {
        await cancelOperation();
      } catch {
        // Ignore
      }
      try {
        await cleanupFailedDownload();
      } catch {
        // Ignore cleanup errors
      }
    })();
    await pendingCleanupRef.current;
  }, [t, clearIntervals, failFlash]);

  // Monitor device connection during active operations.
  // QDL flash stages are excluded: the USB device is busy/resets during Sahara/Firehose (expected).
  useEffect(() => {
    const activeStages: FlashStage[] = isEdlFlash
      ? ['downloading', 'verifying_sha', 'decompressing']
      : ['downloading', 'verifying_sha', 'decompressing',
         'flashing', 'verifying'];
    if (!activeStages.includes(stage)) {
      if (deviceMonitorRef.current) {
        clearInterval(deviceMonitorRef.current);
        deviceMonitorRef.current = null;
      }
      return;
    }

    checkDeviceOrDisconnect();
    deviceMonitorRef.current = window.setInterval(checkDeviceOrDisconnect, POLLING.DEVICE_CHECK);

    return () => {
      if (deviceMonitorRef.current) {
        clearInterval(deviceMonitorRef.current);
        deviceMonitorRef.current = null;
      }
    };
  }, [stage, device.path, isEdlFlash, handleDeviceDisconnectedInternal, checkDeviceOrDisconnect]);

  /** Handle custom image flow (decompress if needed, then flash) */
  async function handleCustomImage(customPath: string) {
    try {
      // QDL custom images (.tar) flash directly; extraction is internal
      if (isQdlMode) {
        setPhases(buildPhases({ download: false, prepare: true, verify: false }));
        setImagePath(customPath);
        startFlash(customPath);
        return;
      }

      const needsDecompress = await checkNeedsDecompression(customPath);
      setPhases(buildPhases({ download: false, prepare: needsDecompress, verify: !skipVerifyRef.current }));

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
      const raw = getErrorMessage(err, '');
      if (deviceDisconnectedRef.current && /cancel/i.test(raw)) return;
      failFlash(raw || t('error.decompressionFailed'));
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
          failFlash(prog.error);
          if (intervalRef.current) clearInterval(intervalRef.current);
        }
      } catch {
        // Ignore polling errors
      }
    }, POLLING.DOWNLOAD_PROGRESS);

    try {
      // Use direct_url: it carries the full filename, unlike the extensionless mirror-selector file_url.
      const path = await downloadImage(image.direct_url, image.sha_url);
      setImagePath(path);
      if (intervalRef.current) clearInterval(intervalRef.current);
      startFlash(path);
    } catch (err) {
      if (intervalRef.current) clearInterval(intervalRef.current);

      const errorMsg = getErrorMessage(err, String(err));

      // SHA fetch failed: show modal; the file is downloaded and kept
      if (!deviceDisconnectedRef.current && isShaUnavailableError(errorMsg)) {
        setShowShaWarning(true);
        return;
      }

      if (deviceDisconnectedRef.current && /cancel/i.test(errorMsg)) return;
      failFlash(errorMsg.trim() ? errorMsg : t('error.downloadFailed'));
    }
  }

  /** Start flash with progress polling */
  async function startFlash(path: string) {
    setStage(isQdlMode ? 'extracting' : 'flashing');
    setProgress(0);
    maxProgressRef.current = 0;
    flashWriteSeenRef.current = false;

    intervalRef.current = window.setInterval(async () => {
      try {
        const prog = await getFlashProgress();

        if (prog.is_qdl_mode && prog.qdl_stage) {
          if (prog.qdl_stage === 'sahara' || prog.qdl_stage === 'connecting' || prog.qdl_stage === 'configuring' || prog.qdl_stage === 'provisioning') {
            setStage('qdl_sahara');
          } else if (prog.qdl_stage.startsWith('partition:') || prog.qdl_stage === 'firehose' || prog.qdl_stage === 'patching') {
            setStage('qdl_firehose');
          } else if (prog.qdl_stage === 'complete' || prog.qdl_stage === 'resetting') {
            // Done/resetting: don't trigger a device-disconnect error
            return;
          }

          if (prog.progress_percent >= maxProgressRef.current) {
            maxProgressRef.current = prog.progress_percent;
            setProgress(prog.progress_percent);
          }
        } else {
          // A non-verifying poll means this run is actually writing: open the latch.
          if (!prog.is_verifying) {
            flashWriteSeenRef.current = true;
          }
          // Only honor verify once the write phase of THIS run has been seen, so a
          // stale is_verifying from a previous flash can't jump straight to a full bar.
          const verifying = prog.is_verifying && flashWriteSeenRef.current;

          if (verifying) {
            setStage('verifying');
            if (maxProgressRef.current > 50) {
              maxProgressRef.current = 0;
            }
          }

          // Skip stale verify polls (is_verifying true before any write seen).
          if (verifying || !prog.is_verifying) {
            if (prog.progress_percent >= maxProgressRef.current) {
              maxProgressRef.current = prog.progress_percent;
              setProgress(prog.progress_percent);
            }
          }
        }
        if (prog.error && !deviceDisconnectedRef.current) {
          failFlash(prog.error);
          if (intervalRef.current) clearInterval(intervalRef.current);
        }
      } catch {
        // Ignore polling errors
      }
    }, POLLING.FLASH_PROGRESS);

    try {
      // Pass autoconfig only when a profile was selected (else undefined = unchanged)
      if (isUfsMode) {
        // UFS path: decompressed .img → Sahara → raw Firehose write to UFS
        await flashQdlUfsImage(
          path,
          soc ?? '',
          boardSlug ?? '',
          undefined,
          autoconfigRef.current ?? undefined
        );
      } else if (isQdlMode) {
        // QDL path: TAR archive → extract → Sahara → Firehose
        await flashQdlImage(path, undefined, autoconfigRef.current ?? undefined);
      } else {
        await flashImage(path, device.path, !skipVerifyRef.current, autoconfigRef.current ?? undefined);
      }
      if (intervalRef.current) clearInterval(intervalRef.current);
      setStage('complete');
      setProgress(100);
      setFlashFailureCount(0);
      // QDL: backend handles temp dir cleanup; don't delete the source TAR
      if (!isQdlMode) {
        await cleanupImageSafely(path, image.is_custom);
      }
    } catch (err) {
      if (intervalRef.current) clearInterval(intervalRef.current);

      const rawError = getErrorMessage(err, String(err));
      // User-initiated cancel: handleCancel owns navigation/messaging.
      if (userCancelledRef.current) return;
      // Cancel-shaped rejection triggered by the disconnect handler's own cancelOperation:
      // the generic disconnect message is already on screen and more truthful.
      const causedByDisconnectCancel = deviceDisconnectedRef.current && /cancel/i.test(rawError);
      if (!causedByDisconnectCancel) {
        failFlash(translateFlashError(rawError, t));
      }

      // Increment failure count for cached (non-custom) images; skip on disconnect
      // so a card pull doesn't burn down a good cached image.
      if (!image.is_custom && !isQdlMode && !deviceDisconnectedRef.current) {
        const currentCount = getFlashFailureCount() + 1;
        setFlashFailureCount(currentCount);

        // Drop cached image after too many failures (possibly corrupted)
        if (currentCount >= CACHE.MAX_FLASH_FAILURES) {
          try {
            await forceDeleteCachedImage(path);
            setFlashFailureCount(0);
          } catch {
            // Ignore deletion errors
          }
        }
      }

      if (!isQdlMode && !deviceDisconnectedRef.current) {
        await cleanupImageSafely(path, image.is_custom);
      }
    }
  }

  /** Authorization flow - entry point for the operation */
  async function handleAuthorization() {
    setStage('authorizing');
    setProgress(0);
    setError(null);
    shownErrorRef.current = null;
    userCancelledRef.current = false;

    try {
      try {
        skipVerifyRef.current = await getSkipVerify();
      } catch {
        skipVerifyRef.current = false;
      }

      // EDL (QDL/UFS) skips block-device authorization (USB access handled by OS)
      if (!isEdlFlash) {
        const authorized = await requestWriteAuthorization(device.path);
        if (!authorized) {
          failFlash(t('error.authCancelled'));
          return;
        }
      }

      if (image.is_custom && image.custom_path) {
        await handleCustomImage(image.custom_path);
      } else {
        // A cache hit returns the decompressed .img immediately, skipping download + prepare.
        let cached = false;
        if (!isQdlMode) {
          try {
            const key = armbianIdentityKey(image.direct_url);
            if (key) {
              const list = await listCachedImages();
              cached = list.some((c) => armbianIdentityKey(c.filename) === key);
            }
          } catch {
            cached = false;
          }
        }
        setPhases(
          buildPhases({
            download: !cached,
            prepare: isQdlMode || (!cached && isCompressedImage(image.direct_url)),
            verify: !isEdlFlash && !skipVerifyRef.current,
          })
        );
        startDownload();
      }
    } catch (err) {
      failFlash(getErrorMessage(err, t('error.authFailed')));
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
    userCancelledRef.current = true;
    try {
      await cancelOperation();
      if (intervalRef.current) clearInterval(intervalRef.current);
      // EDL: stay put so the blocking flash command can detect cancel and clean up
      if (isEdlFlash) {
        setStage('authorizing');
        setProgress(0);
        setError(t('flash.cancel'));
        setStage('error');
      } else {
        await cleanupImageSafely(imagePath, image.is_custom);
        onBack();
      }
    } catch {
      // Ignore
    }
  };

  const handleRetry = async () => {
    // Let the disconnect handler's cancel/cleanup land before restarting,
    // so a stale is_cancelled can't race the new flash's reset.
    if (pendingCleanupRef.current) {
      try {
        await pendingCleanupRef.current;
      } catch {
        // Ignore
      }
      pendingCleanupRef.current = null;
    }
    setError(null);
    deviceDisconnectedRef.current = false;
    userCancelledRef.current = false;
    shownErrorRef.current = null;

    if (imagePath) {
      // EDL: skip block-device authorization (USB access handled by OS). The TAR path
      // re-extracts (prepare), but a UFS .img is already decompressed and ready.
      if (isEdlFlash) {
        setPhases(buildPhases({ download: false, prepare: isQdlMode, verify: false }));
        startFlash(imagePath);
        return;
      }
      // The image is already downloaded/decompressed: only write (and maybe verify) remain.
      setPhases(buildPhases({ download: false, prepare: false, verify: !skipVerifyRef.current }));
      // Re-authorize before re-flashing the existing image
      setStage('authorizing');
      try {
        const authorized = await requestWriteAuthorization(device.path);
        if (!authorized) {
          failFlash(t('error.authCancelled'));
          return;
        }
        startFlash(imagePath);
      } catch (err) {
        failFlash(getErrorMessage(err, t('error.authFailed')));
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
      const raw = getErrorMessage(err, '');
      if (deviceDisconnectedRef.current && /cancel/i.test(raw)) return;
      failFlash(raw || t('error.decompressionFailed'));
    }
  };

  const handleShaWarningCancel = async () => {
    setShowShaWarning(false);
    await cleanupFailedDownload();
    onBack();
  };

  return {
    stage,
    phases,
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
