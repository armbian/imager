// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useState, useCallback, useEffect, useRef, lazy, Suspense } from 'react';
import { useTranslation } from 'react-i18next';
import { Header, HomePage, WelcomePage } from './components/layout';
import { ArmbianBoardModal } from './components/modals';
import { FlashProgress } from './components/flash';
import { CacheManagerModal, SettingsPage } from './components/settings';
import { selectCustomImage, detectBoardFromFilename, classifyCustomImage, logInfo, logWarn, getArmbianRelease, getBoards, getSystemInfo, getCachedBoardImage, checkNeedsDecompression, decompressCustomImage } from './hooks/useTauri';
import { useDeviceMonitor } from './hooks/useDeviceMonitor';
import { useConnectivity } from './hooks/useConnectivity';
import { ToastProvider, useToasts } from './hooks/useToasts';
import { useModalExitAnimation } from './hooks/useModalExitAnimation';
import { UpdateProvider } from './contexts/UpdateContext';
import { useMotion } from './contexts/MotionContext';
import { getArmbianBoardDetection, getShowWelcome, getAutoconfigProfile } from './hooks/useSettings';
import { EVENTS, SLUGS, VENDOR, IMAGE_VARIANT, LOCAL_SOURCE_LABEL, UI, SETTINGS, SETTINGS_VIEW, PLATFORM } from './config';
import { IMAGE_FORMAT, IMAGE_STORAGE, isEdlImage } from './types';
import { presetIsEmpty } from './config/autoconfig';
import { DEFAULT_COLOR, buildLocalImage, buildLocalBoard, localManufacturer, focusAfterInput } from './utils';
import type { BoardInfo, ImageInfo, BlockDevice, SelectionStep, Manufacturer, ArmbianDetectionOutcome, ArmbianReleaseInfo, AutoconfigConfig, CustomImageInfo, FlashExit, LeaveGuard, SettingsView } from './types';
import './styles/index.css';

// Debug-only panel; the define folds to false in release builds, which drops the chunk.
const DevScenarios = __DEV_SCENARIOS__ ? lazy(() => import('./components/dev/DevScenarios')) : null;

function App() {
  return (
    <ToastProvider>
      <UpdateProvider>
        <AppContent />
      </UpdateProvider>
    </ToastProvider>
  );
}

/** Main application content, must be inside ToastProvider to use useToasts() */
function AppContent() {
  const { t } = useTranslation();
  const [isFlashing, setIsFlashing] = useState(false);
  const [settledFlashExit, setSettledFlashExit] = useState<FlashExit | null>(null);
  const [selectionEpoch, setSelectionEpoch] = useState(0);
  // null until the stored preference is read, so neither page flashes at startup
  const [welcomeState, setShowWelcome] = useState<boolean | null>(null);
  const welcomeKnown = welcomeState !== null;
  const showWelcome = welcomeState ?? true;
  // One-shot entrance animation window: true only while the main UI staggers in
  const [entering, setEntering] = useState(false);
  const prevShowWelcomeRef = useRef(showWelcome);
  const [selectedManufacturer, setSelectedManufacturer] = useState<Manufacturer | null>(null);
  const [selectedBoard, setSelectedBoard] = useState<BoardInfo | null>(null);
  const [selectedImage, setSelectedImage] = useState<ImageInfo | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<BlockDevice | null>(null);
  // Opt-in autoconfig profile id picked at flash time; null means unchanged behaviour.
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [autoconfig, setAutoconfig] = useState<AutoconfigConfig | null>(null);

  const { showSuccess, showError } = useToasts();

  const { isOnline } = useConnectivity();
  const prevOnlineRef = useRef<boolean | null>(null);

  // Armbian board detection state
  const [armbianInfo, setArmbianInfo] = useState<ArmbianReleaseInfo | null>(null);
  const [detectedBoard, setDetectedBoard] = useState<BoardInfo | null>(null);
  const [armbianBoardImageUrl, setArmbianBoardImageUrl] = useState<string | null>(null);
  const [showArmbianModal, setShowArmbianModal] = useState(false);
  // Board queued by silent ('auto') detection, applied once the welcome screen is dismissed.
  const [pendingAutoSelect, setPendingAutoSelect] = useState<BoardInfo | null>(null);
  const [showCacheManager, setShowCacheManager] = useState(false);
  const armbianCheckRef = useRef(false); // Prevent double execution in Strict Mode

  const { reduced } = useMotion();
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const leaveGuardRef = useRef<LeaveGuard | null>(null);
  const gearRef = useRef<HTMLButtonElement>(null);
  const focusReturnRef = useRef<HTMLElement | null>(null);
  const restoreFocusRef = useRef(false);
  const pointerInputRef = useRef(false);

  const [homeReturning, setHomeReturning] = useState(false);

  const dropSettings = useCallback(() => {
    leaveGuardRef.current = null;
    setSettings(null);
  }, []);
  const startHomeReturn = useCallback(() => setHomeReturning(!reduced), [reduced]);
  const { isExiting: settingsExiting, handleClose: closeSettingsNow } = useModalExitAnimation({
    onClose: dropSettings,
    onExiting: startHomeReturn,
    duration: reduced ? 0 : UI.SETTINGS_PAGE.EXIT_MS,
  });

  // The guard shows the editor's own "Discard changes?" dialog and resolves false on Cancel.
  const guardedLeave = useCallback(async (leave: () => void) => {
    const guard = leaveGuardRef.current;
    if (guard && !(await guard())) return;
    leave();
  }, []);

  const registerLeaveGuard = useCallback((guard: LeaveGuard | null) => {
    leaveGuardRef.current = guard;
  }, []);

  const openSettings = useCallback(
    (view: SettingsView) => {
      if (!settings) {
        focusReturnRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        restoreFocusRef.current = true;
        setHomeReturning(false);
        setSettings(view);
        return;
      }
      guardedLeave(() => {
        leaveGuardRef.current = null;
        setSettings(view);
      });
    },
    [settings, guardedLeave]
  );

  const requestCloseSettings = useCallback(() => {
    guardedLeave(closeSettingsNow);
  }, [guardedLeave, closeSettingsNow]);

  const requestSettingsView = useCallback(
    (view: SettingsView) => {
      guardedLeave(() => {
        leaveGuardRef.current = null;
        setSettings((current) => (current ? view : current));
      });
    },
    [guardedLeave]
  );

  // Picking a cached image to reuse sends the user straight back to the flow.
  useEffect(() => {
    if (!settings) return;
    window.addEventListener(EVENTS.CACHE_IMAGE_REUSE, closeSettingsNow);
    return () => window.removeEventListener(EVENTS.CACHE_IMAGE_REUSE, closeSettingsNow);
  }, [settings, closeSettingsNow]);

  const settingsCovering = !!settings && !settingsExiting;

  // Last input modality, so focus returned after a mouse close does not draw a keyboard ring.
  useEffect(() => {
    const onPointer = () => {
      pointerInputRef.current = true;
    };
    const onKey = () => {
      pointerInputRef.current = false;
    };
    window.addEventListener('pointerdown', onPointer, true);
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('pointerdown', onPointer, true);
      window.removeEventListener('keydown', onKey, true);
    };
  }, []);

  // Runs after the commit that lifts inert from the home layer, so its controls can take focus again.
  useEffect(() => {
    if (settings || !restoreFocusRef.current) return;
    restoreFocusRef.current = false;
    const target = focusReturnRef.current;
    focusReturnRef.current = null;
    const next = target?.isConnected && target !== document.body ? target : gearRef.current;
    if (!next) return;
    focusAfterInput(next, pointerInputRef.current);
  }, [settings]);

  // Skip the landing page on startup when the user disabled it; defaults to showing it
  useEffect(() => {
    getShowWelcome()
      .then((show) => {
        // A first known "off" is a plain start, not a welcome->main transition
        if (!show) prevShowWelcomeRef.current = false;
        setShowWelcome(!!show);
      })
      .catch(() => {
        setShowWelcome(true);
      });
  }, []);

  // Fire the entrance animation once on the welcome->main transition
  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    if (prevShowWelcomeRef.current && !showWelcome) {
      setEntering(true);
      timeoutId = setTimeout(() => setEntering(false), UI.ENTRANCE_MS);
    }
    prevShowWelcomeRef.current = showWelcome;
    return () => clearTimeout(timeoutId);
  }, [showWelcome]);

  // Clear selected device if disconnected, only when not flashing
  useDeviceMonitor(
    selectedDevice,
    !!selectedImage && isEdlImage(selectedImage),
    useCallback(() => setSelectedDevice(null), []),
    !isFlashing
  );

  // Receive the opt-in autoconfig profile id chosen in the DevicePanel confirm view
  useEffect(() => {
    const handler = (e: Event) => {
      const id = (e as CustomEvent<{ id: string | null }>).detail?.id ?? null;
      setSelectedProfileId(id);
    };
    window.addEventListener(EVENTS.AUTOCONFIG_PROFILE_SELECTED, handler);
    return () => window.removeEventListener(EVENTS.AUTOCONFIG_PROFILE_SELECTED, handler);
  }, []);

  // Resolve the picked profile id to its config; null when no profile is selected
  useEffect(() => {
    if (!selectedProfileId) {
      setAutoconfig(null);
      return;
    }
    let cancelled = false;
    getAutoconfigProfile(selectedProfileId)
      .then((profile) => {
        // An all-defaults profile renders an empty preset: nothing to prepare or inject
        const config = profile?.config;
        if (!cancelled) setAutoconfig(config && !presetIsEmpty(config) ? config : null);
      })
      .catch(() => {
        if (!cancelled) setAutoconfig(null);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedProfileId]);

  useEffect(() => {
    // Toast on reconnect, but not on initial mount
    if (prevOnlineRef.current === false && isOnline) {
      showSuccess(t('home.connectionRestored'));
    }

    // Going offline drops any API-driven selection (manufacturer/board/API image) back to the offline
    // layout, but preserves local custom/cached images (is_custom), which work offline.
    if (
      prevOnlineRef.current === true &&
      !isOnline &&
      selectedManufacturer &&
      !selectedImage?.is_custom
    ) {
      resetSelectionsFrom('manufacturer');
    }

    prevOnlineRef.current = isOnline;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOnline, showSuccess, t]);

  // Auto-select manufacturer and board from a detected Armbian board
  const autoSelectBoard = useCallback(async (board: BoardInfo) => {
    try {
      const manufacturer: Manufacturer = {
        id: board.vendor || VENDOR.FALLBACK_ID,
        name: board.vendor_name || VENDOR.FALLBACK_NAME,
        color: DEFAULT_COLOR,
        boardCount: 1,
      };

      // Auto-selection fills the flow but never dismisses the landing; that is gated on the welcome screen.
      setSelectedManufacturer(manufacturer);
      setSelectedBoard(board);

      setSelectedImage(null);
      setSelectedDevice(null);

      logInfo('app', `Auto-selected: ${manufacturer.name} → ${board.name} (${board.slug})`);
    } catch (err) {
      logWarn('app', `Failed to auto-select board: ${err}`);
    }
  }, []);

  // Apply a silent ('auto') detection only once past the welcome screen, so it auto-completes
  // the flow instead of skipping the landing.
  useEffect(() => {
    if (!showWelcome && pendingAutoSelect) {
      autoSelectBoard(pendingAutoSelect);
      setPendingAutoSelect(null);
    }
  }, [showWelcome, pendingAutoSelect, autoSelectBoard]);

  // Read the Armbian host and either show the modal or auto-select, as the detection setting says
  const detectArmbianBoard = useCallback(async (): Promise<ArmbianDetectionOutcome> => {
    const { MODAL, AUTO, DISABLED } = SETTINGS.ARMBIAN_DETECTION_MODES;
    const info = await getArmbianRelease();
    if (!info) {
      logInfo('app', 'Not running on Armbian system');
      return 'notArmbian';
    }

    setArmbianInfo(info);

    const detectionMode = await getArmbianBoardDetection();
    if (detectionMode === DISABLED) {
      return DISABLED;
    }

    const boards = await getBoards();
    const matchedBoard = boards.find((b) => b.slug === info.board);

    if (!matchedBoard) {
      logWarn('app', `Board ${info.board} not found in API, skipping auto-selection`);
      return 'unknownBoard';
    }

    logInfo('app', `Found matching board in API: ${matchedBoard.name}`);

    setDetectedBoard(matchedBoard);

    // Load board image from local cache (downloads if online and uncached)
    try {
      const cachedDataUri = await getCachedBoardImage(matchedBoard.slug);
      setArmbianBoardImageUrl(cachedDataUri);
      if (cachedDataUri) {
        logInfo('app', 'Board image loaded from cache');
      }
    } catch (err) {
      logWarn('app', `Failed to get board image: ${err}`);
    }

    if (detectionMode === MODAL) {
      setShowArmbianModal(true);
      return MODAL;
    }
    if (detectionMode === AUTO) {
      // Queue the silent auto-selection; it runs after the welcome screen, never skipping it.
      setPendingAutoSelect(matchedBoard);
      return AUTO;
    }
    return DISABLED;
  }, []);

  // On startup, detect an Armbian host and either show the modal or auto-select
  useEffect(() => {
    const checkArmbianSystem = async () => {
      try {
        if (armbianCheckRef.current) return;

        // Armbian detection is Linux-only
        const systemInfo = await getSystemInfo();
        if (systemInfo.platform !== PLATFORM.LINUX) {
          armbianCheckRef.current = true;
          logInfo('app', `Skipping Armbian detection on ${systemInfo.platform}`);
          return;
        }

        // Skip if offline, board matching requires API data; don't set ref so it retries when online
        if (!isOnline) {
          logInfo('app', 'Skipping Armbian board detection: offline');
          return;
        }

        armbianCheckRef.current = true;
        await detectArmbianBoard();
      } catch (err) {
        logWarn('app', `Failed to check for Armbian system: ${err}`);
      }
    };

    checkArmbianSystem();
  }, [isOnline, detectArmbianBoard]);

  // Dev scenarios only: run detection again for a simulated host, on any platform
  const rerunArmbianDetection = useCallback(async (): Promise<ArmbianDetectionOutcome> => {
    armbianCheckRef.current = true;
    setDetectedBoard(null);
    setArmbianBoardImageUrl(null);
    try {
      return await detectArmbianBoard();
    } catch (err) {
      logWarn('app', `Failed to check for Armbian system: ${err}`);
      return 'failed';
    }
  }, [detectArmbianBoard]);

  // Reuse a cached image from the Cache Manager: select its board and image
  useEffect(() => {
    const handler = async (e: Event) => {
      const { path, filename, size, boardSlug, boardName } = (e as CustomEvent).detail;

      logInfo('app', `Reusing cached image: ${filename}`);

      let matchedBoard: BoardInfo | null = null;
      try {
        matchedBoard = await detectBoardFromFilename(filename);
        if (matchedBoard) {
          logInfo('app', `Detected board from cached filename: ${matchedBoard.name}`);
        }
      } catch {
        // Ignore detection errors
      }

      let imagePath = path;
      try {
        const needsDecompress = await checkNeedsDecompression(path);
        if (needsDecompress) {
          logInfo('app', `Cached image needs decompression: ${filename}`);
          imagePath = await decompressCustomImage(path);
        }
      } catch (err) {
        logWarn('app', `Failed to check/decompress cached image: ${err}`);
        // Continue with original path
      }

      const cachedImage = buildLocalImage({
        variant: IMAGE_VARIANT.CACHED,
        name: filename,
        size,
        path: imagePath,
        format: IMAGE_FORMAT.SD,
      });

      resetSelectionsFrom('board');

      // API-matched board, else fall back to cache metadata (boardSlug/boardName
      // parsed from the filename) since the API match fails when offline.
      const hasCacheMetadata = boardSlug && boardSlug !== SLUGS.CACHED;
      const displayBoard = matchedBoard || buildLocalBoard({
        slug: boardSlug || SLUGS.CACHED,
        name: boardName || t('custom.customImage'),
        vendor: hasCacheMetadata ? SLUGS.DETECTED : SLUGS.CACHED,
        vendorName: hasCacheMetadata ? (boardName || VENDOR.UNKNOWN_NAME) : LOCAL_SOURCE_LABEL[IMAGE_VARIANT.CACHED],
      });

      setSelectedManufacturer(localManufacturer(displayBoard));
      setSelectedBoard(displayBoard);
      setSelectedImage(cachedImage);
    };

    window.addEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
    return () => window.removeEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
  }, [t]);

  // Reset a step and all downstream selections, which become invalid when it changes
  function resetSelectionsFrom(step: SelectionStep) {
    const steps: SelectionStep[] = ['manufacturer', 'board', 'image', 'device'];
    const stepIndex = steps.indexOf(step);

    if (stepIndex <= 0) setSelectedManufacturer(null);
    if (stepIndex <= 1) setSelectedBoard(null);
    if (stepIndex <= 2) setSelectedImage(null);
    if (stepIndex <= 3) {
      setSelectedDevice(null);
      // The profile picker belongs to the device step; drop the opt-in selection.
      setSelectedProfileId(null);
    }
  }

  function handleManufacturerSelect(manufacturer: Manufacturer) {
    setSelectedManufacturer(manufacturer);
    resetSelectionsFrom('board');
  }

  function handleBoardSelect(board: BoardInfo) {
    setSelectedBoard(board);
    resetSelectionsFrom('image');
  }

  function handleImageSelect(image: ImageInfo) {
    setSelectedImage(image);
    resetSelectionsFrom('device');
  }

  // Reveals the inline confirm summary; does not start flashing yet
  function handleDeviceSelect(device: BlockDevice) {
    setSelectedDevice(device);
  }

  // Confirm the inline summary: begin flashing the picked device
  function handleConfirmFlash() {
    setIsFlashing(true);
  }

  // Cancel the inline confirm: drop back to the device list
  function handleClearDevice() {
    setSelectedDevice(null);
    setSelectedProfileId(null);
  }

  async function handleCustomImage() {
    try {
      const result = await selectCustomImage();
      if (result) {
        await applyCustomImage(result);
      }
    } catch (err) {
      logWarn('app', `Failed to select custom image: ${err}`);
      showError(t('custom.selectError'));
    }
  }

  async function applyCustomImage(result: CustomImageInfo) {
    // One backend call classifies the picked file: matched board, QDL TAR, UFS build slug and profile support.
    const {
      board: detectedBoard,
      is_qdl: isQdl,
      ufs_board_slug: ufsBoardSlug,
      supports_autoconfig: supportsAutoconfig,
    } = await classifyCustomImage(result.path).catch(() => ({
      board: null,
      is_qdl: false,
      ufs_board_slug: null,
      supports_autoconfig: false,
    }));
    if (detectedBoard) {
      logInfo('app', `Detected board from filename: ${detectedBoard.name} (${detectedBoard.slug})`);
    }
    if (isQdl) {
      logInfo('app', `Custom image detected as QDL archive: ${result.name}`);
    }
    if (ufsBoardSlug) {
      logInfo('app', `Custom image detected as UFS: ${result.name} (board ${ufsBoardSlug})`);
    }
    const format = isQdl ? IMAGE_FORMAT.QDL : IMAGE_FORMAT.BLOCK;

    const customImage = buildLocalImage({
      variant: IMAGE_VARIANT.CUSTOM,
      name: result.name,
      size: result.size,
      path: result.path,
      format,
      storage: ufsBoardSlug ? IMAGE_STORAGE.UFS : null,
      supportsAutoconfig,
    });

    resetSelectionsFrom('manufacturer');

    // API-matched board, else a generic one carrying the UFS registry slug (backend resolves the rest).
    const displayBoard: BoardInfo = detectedBoard ?? buildLocalBoard({
      slug: ufsBoardSlug ?? SLUGS.CUSTOM,
      name: t('custom.customImage'),
      vendor: SLUGS.CUSTOM,
      vendorName: LOCAL_SOURCE_LABEL[IMAGE_VARIANT.CUSTOM],
    });

    setSelectedManufacturer(localManufacturer(displayBoard));
    setSelectedBoard(displayBoard);
    setSelectedImage(customImage);
    setShowWelcome(false);
  }

  function handleComplete() {
    setIsFlashing(false);
    resetSelectionsFrom('manufacturer');
  }

  function handleBackFromFlash() {
    setIsFlashing(false);
    setSelectedDevice(null); // Allow re-selection
    setSelectedProfileId(null);
  }

  function handleReset() {
    resetSelectionsFrom('manufacturer');
  }

  const handleFlashSettled = useCallback((exit: FlashExit | null) => {
    setSettledFlashExit(() => exit);
  }, []);

  async function handleRestartSelection() {
    if (isFlashing) {
      if (!settledFlashExit) return;
      await settledFlashExit();
    }
    resetSelectionsFrom('manufacturer');
    setSelectionEpoch((epoch) => epoch + 1);
  }

  function handleNavigateToStep(step: SelectionStep) {
    // Reset from this step onward so it becomes the active inline panel
    resetSelectionsFrom(step);
  }

  // Confirm the Armbian modal: auto-select the detected board (from cache, no refetch)
  const handleArmbianConfirm = useCallback(async () => {
    if (!detectedBoard) {
      logWarn('app', 'No detected board available for auto-selection');
      setShowArmbianModal(false);
      return;
    }

    await autoSelectBoard(detectedBoard);
    setShowArmbianModal(false);
  }, [detectedBoard, autoSelectBoard]);

  /** Dismiss the Armbian modal and proceed with manual selection */
  const handleArmbianCancel = useCallback(() => {
    logInfo('app', 'User cancelled Armbian board auto-selection');
    setShowArmbianModal(false);
  }, []);

  /** Show a toast when board detection is disabled from the Armbian modal */
  const handleDetectionDisabled = useCallback(() => {
    showError(t('armbian.disabledToast'));
  }, [showError, t]);

  return (
    <div className="app">
      {/* macOS overlay titlebar drag strip; reserved --macos-titlebar-h stays clear of the traffic-light controls */}
      <div className="titlebar-drag" data-tauri-drag-region />
      <Header
        selectedManufacturer={selectedManufacturer}
        selectedBoard={selectedBoard}
        selectedImage={selectedImage}
        selectedDevice={selectedDevice}
        onReset={handleReset}
        onNavigateToStep={handleNavigateToStep}
        isFlashing={isFlashing}
        isOnline={isOnline}
        hideSteps={showWelcome}
        hideSettings={showWelcome || isFlashing}
        hideLogo={showWelcome}
        entering={entering}
        settingsOpen={!!settings && !settingsExiting}
        onOpenSettings={() => openSettings(SETTINGS_VIEW.GENERAL)}
        onCloseSettings={requestCloseSettings}
        settingsButtonRef={gearRef}
      />

      <main
        className={`main-content${
          isFlashing ? '' : showWelcome ? ' main-content--welcome' : ' main-content--home'
        }`}
      >
        {!welcomeKnown && !isFlashing ? null : isFlashing ? (
          selectedBoard && selectedImage && selectedDevice && (
            <FlashProgress
              board={selectedBoard}
              image={selectedImage}
              device={selectedDevice}
              autoconfig={autoconfig}
              onComplete={handleComplete}
              onBack={handleBackFromFlash}
              onSettledChange={DevScenarios ? handleFlashSettled : undefined}
            />
          )
        ) : showWelcome ? (
          <WelcomePage onStart={() => setShowWelcome(false)} />
        ) : (
          <>
          <div
            className={`home-layer${settingsCovering ? ' is-covered' : homeReturning ? ' is-returning' : ''}`}
            onTransitionEnd={(e) => {
              if (e.target === e.currentTarget && e.propertyName === 'transform') setHomeReturning(false);
            }}
            onTransitionCancel={(e) => {
              if (e.target === e.currentTarget && e.propertyName === 'transform') setHomeReturning(false);
            }}
            inert={settingsCovering || undefined}
            aria-hidden={settingsCovering || undefined}
          >
          <HomePage
            key={selectionEpoch}
            selectedManufacturer={selectedManufacturer}
            selectedBoard={selectedBoard}
            selectedImage={selectedImage}
            selectedDevice={selectedDevice}
            onChooseManufacturer={() => resetSelectionsFrom('manufacturer')}
            onChooseBoard={() => resetSelectionsFrom('board')}
            onChooseImage={() => resetSelectionsFrom('image')}
            onChooseDevice={() => resetSelectionsFrom('device')}
            onChooseCustomImage={handleCustomImage}
            onOpenCacheManager={() => setShowCacheManager(true)}
            onSelectManufacturer={handleManufacturerSelect}
            onSelectBoard={handleBoardSelect}
            onSelectImage={handleImageSelect}
            onSelectDevice={handleDeviceSelect}
            onConfirmDevice={handleConfirmFlash}
            onClearDevice={handleClearDevice}
            isOnline={isOnline}
            entering={entering}
          />
          </div>
          {settings && (
            <SettingsPage
              view={settings}
              exiting={settingsExiting}
              onViewChange={requestSettingsView}
              onClose={requestCloseSettings}
              registerLeaveGuard={registerLeaveGuard}
            />
          )}
          </>
        )}
      </main>

      {armbianInfo && (
        <ArmbianBoardModal
          isOpen={showArmbianModal && !showWelcome}
          onClose={handleArmbianCancel}
          onConfirm={handleArmbianConfirm}
          onDetectionDisabled={handleDetectionDisabled}
          armbianInfo={armbianInfo}
          boardInfo={detectedBoard}
          boardImageUrl={armbianBoardImageUrl}
        />
      )}

      {/* Standalone cache manager for offline mode */}
      <CacheManagerModal
        isOpen={showCacheManager}
        onClose={() => setShowCacheManager(false)}
      />

      {DevScenarios && (
        <Suspense fallback={null}>
          <DevScenarios
            hidden={showWelcome}
            isFlashing={isFlashing}
            isConfirming={!isFlashing && selectedDevice !== null}
            onUseCustomImage={applyCustomImage}
            onResetFlow={isFlashing && !settledFlashExit ? null : handleRestartSelection}
            onRunArmbianDetection={isOnline ? rerunArmbianDetection : null}
          />
        </Suspense>
      )}
    </div>
  );
}

export default App;
