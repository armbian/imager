import { useState, useCallback, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Header, HomePage, WelcomePage } from './components/layout';
import { ArmbianBoardModal } from './components/modals';
import { FlashProgress } from './components/flash';
import { CacheManagerModal } from './components/settings';
import { selectCustomImage, detectBoardFromFilename, classifyCustomImage, logInfo, logWarn, getArmbianRelease, getBoards, getSystemInfo, getCachedBoardImage, checkNeedsDecompression, decompressCustomImage } from './hooks/useTauri';
import { useDeviceMonitor } from './hooks/useDeviceMonitor';
import { useConnectivity } from './hooks/useConnectivity';
import { ToastProvider, useToasts } from './hooks/useToasts';
import { UpdateProvider } from './contexts/UpdateContext';
import { getArmbianBoardDetection, getShowWelcome, getAutoconfigProfile } from './hooks/useSettings';
import { AUTOCONFIG_PROFILE_SELECTED_EVENT } from './components/layout/DevicePanel';
import { EVENTS, SLUGS, VENDOR, IMAGE_VARIANT } from './config';
import type { BoardInfo, ImageInfo, BlockDevice, SelectionStep, Manufacturer, ArmbianReleaseInfo, AutoconfigConfig } from './types';
import './styles/index.css';

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
  // Shown on every launch until the user hits "Start now"
  const [showWelcome, setShowWelcome] = useState(true);
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

  // Skip the landing page on startup when the user disabled it; defaults to showing it
  useEffect(() => {
    getShowWelcome()
      .then((show) => {
        if (!show) setShowWelcome(false);
      })
      .catch(() => {
        // Keep showing the welcome page on failure
      });
  }, []);

  // Fire the entrance animation once on the welcome->main transition
  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    if (prevShowWelcomeRef.current && !showWelcome) {
      setEntering(true);
      // Match the longest staggered animation so the class clears before hover state
      timeoutId = setTimeout(() => setEntering(false), 1100);
    }
    prevShowWelcomeRef.current = showWelcome;
    return () => clearTimeout(timeoutId);
  }, [showWelcome]);

  // Clear selected device if disconnected, only when not flashing
  useDeviceMonitor(
    selectedDevice,
    useCallback(() => setSelectedDevice(null), []),
    !isFlashing
  );

  // Receive the opt-in autoconfig profile id chosen in the DevicePanel confirm view
  useEffect(() => {
    const handler = (e: Event) => {
      const id = (e as CustomEvent<{ id: string | null }>).detail?.id ?? null;
      setSelectedProfileId(id);
    };
    window.addEventListener(AUTOCONFIG_PROFILE_SELECTED_EVENT, handler);
    return () => window.removeEventListener(AUTOCONFIG_PROFILE_SELECTED_EVENT, handler);
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
        if (!cancelled) setAutoconfig(profile?.config ?? null);
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
        name: board.vendor_name || 'Other',
        color: 'slate',
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

  // On startup, detect an Armbian host and either show the modal or auto-select
  useEffect(() => {
    const checkArmbianSystem = async () => {
      try {
        if (armbianCheckRef.current) return;

        // Armbian detection is Linux-only
        const systemInfo = await getSystemInfo();
        if (systemInfo.platform !== 'linux') {
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

        const info = await getArmbianRelease();
        if (!info) {
          logInfo('app', 'Not running on Armbian system');
          return;
        }

        setArmbianInfo(info);

        const detectionMode = await getArmbianBoardDetection();
        if (detectionMode === 'disabled') {
          return;
        }

        const boards = await getBoards();
        const matchedBoard = boards.find((b) => b.slug === info.board);

        if (!matchedBoard) {
          logWarn('app', `Board ${info.board} not found in API, skipping auto-selection`);
          return;
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

        if (detectionMode === 'modal') {
          setShowArmbianModal(true);
        } else if (detectionMode === 'auto') {
          // Queue the silent auto-selection; it runs after the welcome screen, never skipping it.
          setPendingAutoSelect(matchedBoard);
        }
      } catch (err) {
        logWarn('app', `Failed to check for Armbian system: ${err}`);
      }
    };

    checkArmbianSystem();
  }, [isOnline]);

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

      const cachedImage: ImageInfo = {
        release: 'Cached',
        distro_release: filename,
        kernel_branch: '',
        kernel_version: '',
        image_variant: IMAGE_VARIANT.CACHED,
        preinstalled_application: '',
        promoted: false,
        file_url: '',
        direct_url: '',
        sha_url: null,
        file_size: size,
        stability: 'stable',
        format: 'sd',
        companions: [],
        display_variants: [],
        is_custom: true,
        custom_path: imagePath,
      };

      resetSelectionsFrom('board');

      // API-matched board, else fall back to cache metadata (boardSlug/boardName
      // parsed from the filename) since the API match fails when offline.
      const hasCacheMetadata = boardSlug && boardSlug !== SLUGS.CACHED;
      const displayBoard = matchedBoard || {
        slug: boardSlug || SLUGS.CACHED,
        name: boardName || t('custom.customImage'),
        vendor: hasCacheMetadata ? SLUGS.DETECTED : SLUGS.CACHED,
        vendor_name: hasCacheMetadata ? (boardName || 'Unknown') : 'Cached',
        support_tier: 'community',
        image_count: 1,
        has_desktop: false,
        promoted: false,
      };

      setSelectedManufacturer({
        id: displayBoard.vendor,
        name: displayBoard.vendor_name,
        color: '#6b7280',
        boardCount: 1,
      });
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
        // One backend call classifies the picked file: matched board, QDL TAR, and UFS build slug.
        const { board: detectedBoard, is_qdl: isQdl, ufs_board_slug: ufsBoardSlug } =
          await classifyCustomImage(result.path).catch(() => ({
            board: null,
            is_qdl: false,
            ufs_board_slug: null,
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
        const flashMethod = isQdl ? 'qdl' : 'block';

        const customImage: ImageInfo = {
          release: 'Custom',
          distro_release: result.name,
          kernel_branch: '',
          kernel_version: '',
          image_variant: IMAGE_VARIANT.CUSTOM,
          preinstalled_application: '',
          promoted: false,
          file_url: '',
          direct_url: '',
          sha_url: null,
          file_size: result.size,
          stability: 'stable',
          format: flashMethod,
          storage: ufsBoardSlug ? 'ufs' : null,
          companions: [],
          display_variants: [],
          is_custom: true,
          custom_path: result.path,
        };

        resetSelectionsFrom('manufacturer');

        // API-matched board, else a generic one carrying the UFS registry slug (backend resolves the rest).
        const displayBoard: BoardInfo = detectedBoard ?? {
          slug: ufsBoardSlug ?? SLUGS.CUSTOM,
          name: t('custom.customImage'),
          vendor: SLUGS.CUSTOM,
          vendor_name: 'Custom',
          support_tier: 'community',
          image_count: 1,
          has_desktop: false,
          promoted: false,
        };

        setSelectedManufacturer({
          id: displayBoard.vendor,
          name: displayBoard.vendor_name,
          color: '#6b7280',
          boardCount: 1,
        });
        setSelectedBoard(displayBoard);
        setSelectedImage(customImage);
        // A custom image bypasses the landing and enters the flow directly
        setShowWelcome(false);
      }
    } catch (err) {
      console.error('Failed to select custom image:', err);
    }
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
      {/* macOS overlay titlebar drag strip; reserved 42px stays clear of the traffic-light controls */}
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
      />

      <main
        className={`main-content${
          isFlashing ? '' : showWelcome ? ' main-content--welcome' : ' main-content--home'
        }`}
      >
        {isFlashing ? (
          selectedBoard && selectedImage && selectedDevice && (
            <FlashProgress
              board={selectedBoard}
              image={selectedImage}
              device={selectedDevice}
              autoconfig={autoconfig}
              onComplete={handleComplete}
              onBack={handleBackFromFlash}
            />
          )
        ) : showWelcome ? (
          <WelcomePage onStart={() => setShowWelcome(false)} />
        ) : (
          <HomePage
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
    </div>
  );
}

export default App;
