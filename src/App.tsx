import { useState, useCallback, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Header, HomePage } from './components/layout';
import { ManufacturerModal, BoardModal, ImageModal, DeviceModal, ArmbianBoardModal } from './components/modals';
import { FlashProgress } from './components/flash';
import { SettingsButton } from './components/settings';
import { selectCustomImage, detectBoardFromFilename, logInfo, logWarn, getArmbianRelease, getBoards, getSystemInfo, getBoardImageUrl } from './hooks/useTauri';
import { useDeviceMonitor } from './hooks/useDeviceMonitor';
import { ToastProvider, useToasts } from './hooks/useToasts';
import { getArmbianBoardDetection } from './hooks/useSettings';
import type { BoardInfo, ImageInfo, BlockDevice, ModalType, SelectionStep, Manufacturer, ArmbianReleaseInfo } from './types';
import './styles/index.css';

function App() {
  return (
    <ToastProvider>
      <AppContent />
    </ToastProvider>
  );
}

/** Main application content — must be inside ToastProvider to use useToasts() */
function AppContent() {
  const { t } = useTranslation();
  const [isFlashing, setIsFlashing] = useState(false);
  const [activeModal, setActiveModal] = useState<ModalType>('none');
  const [selectedManufacturer, setSelectedManufacturer] = useState<Manufacturer | null>(null);
  const [selectedBoard, setSelectedBoard] = useState<BoardInfo | null>(null);
  const [selectedImage, setSelectedImage] = useState<ImageInfo | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<BlockDevice | null>(null);

  // Toast notifications
  const { showError } = useToasts();

  // Armbian board detection state
  const [armbianInfo, setArmbianInfo] = useState<ArmbianReleaseInfo | null>(null);
  const [detectedBoard, setDetectedBoard] = useState<BoardInfo | null>(null); // Cache matched board
  const [armbianBoardImageUrl, setArmbianBoardImageUrl] = useState<string | null>(null); // Preloaded board image
  const [showArmbianModal, setShowArmbianModal] = useState(false);
  const armbianCheckRef = useRef(false); // Prevent double execution in Strict Mode

  // Monitor selected device - clear if disconnected (only when not flashing)
  useDeviceMonitor(
    selectedDevice,
    useCallback(() => setSelectedDevice(null), []),
    !isFlashing
  );

  /**
   * Auto-select board based on Armbian detection
   * Sets manufacturer and board without user interaction
   */
  const autoSelectBoard = useCallback(async (board: BoardInfo) => {
    try {
      // Create manufacturer from board vendor
      const manufacturer: Manufacturer = {
        id: board.vendor || 'other',
        name: board.vendor_name || 'Other',
        color: 'slate',
        boardCount: 1,
      };

      setSelectedManufacturer(manufacturer);
      setSelectedBoard(board);

      // Reset image and device selections (user still needs to select these)
      setSelectedImage(null);
      setSelectedDevice(null);

      logInfo('app', `Auto-selected: ${manufacturer.name} → ${board.name} (${board.slug})`);
    } catch (err) {
      logWarn('app', `Failed to auto-select board: ${err}`);
    }
  }, []);

  /**
   * Check for Armbian system on app startup
   * Detects if running on Armbian and either shows modal or auto-selects based on settings
   */
  useEffect(() => {
    const checkArmbianSystem = async () => {
      try {
        // Check platform BEFORE setting ref - Armbian detection is Linux-only
        const systemInfo = await getSystemInfo();
        if (systemInfo.platform !== 'linux') {
          logInfo('app', `Skipping Armbian detection on ${systemInfo.platform}`);
          return;
        }

        // Prevent double execution in React Strict Mode (only for Linux)
        if (armbianCheckRef.current) return;
        armbianCheckRef.current = true;

        const info = await getArmbianRelease();
        if (!info) {
          logInfo('app', 'Not running on Armbian system');
          return;
        }

        setArmbianInfo(info);

        // Get detection mode from settings
        const detectionMode = await getArmbianBoardDetection();

        // Skip if disabled
        if (detectionMode === 'disabled') {
          return;
        }

        // Fetch boards to find matching board
        const boards = await getBoards();
        const matchedBoard = boards.find((b) => b.slug === info.board);

        if (!matchedBoard) {
          logWarn('app', `Board ${info.board} not found in API, skipping auto-selection`);
          return;
        }

        logInfo('app', `Found matching board in API: ${matchedBoard.name}`);

        // Cache the matched board for use in modal
        setDetectedBoard(matchedBoard);

        // Preload board image before showing modal
        try {
          const baseUrl = await getBoardImageUrl(matchedBoard.slug);
          if (baseUrl) {
            const imgUrl = baseUrl.replace('/272/', '/480/');
            // Preload image
            const img = new Image();
            img.onload = () => {
              setArmbianBoardImageUrl(imgUrl);
              logInfo('app', `Board image preloaded: ${imgUrl}`);
            };
            img.onerror = () => {
              logWarn('app', `Failed to preload board image: ${imgUrl}`);
              setArmbianBoardImageUrl(null);
            };
            img.src = imgUrl;
          }
        } catch (err) {
          logWarn('app', `Failed to get board image URL: ${err}`);
        }

        if (detectionMode === 'modal') {
          // Show confirmation modal
          setShowArmbianModal(true);
        } else if (detectionMode === 'auto') {
          // Auto-select without confirmation
          await autoSelectBoard(matchedBoard);
        }
      } catch (err) {
        logWarn('app', `Failed to check for Armbian system: ${err}`);
      }
    };

    checkArmbianSystem();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /**
   * Reset selections from a given step onwards.
   * When user changes a selection, downstream selections become invalid.
   */
  function resetSelectionsFrom(step: SelectionStep) {
    const steps: SelectionStep[] = ['manufacturer', 'board', 'image', 'device'];
    const stepIndex = steps.indexOf(step);

    if (stepIndex <= 0) setSelectedManufacturer(null);
    if (stepIndex <= 1) setSelectedBoard(null);
    if (stepIndex <= 2) setSelectedImage(null);
    if (stepIndex <= 3) setSelectedDevice(null);
  }

  function handleManufacturerSelect(manufacturer: Manufacturer) {
    setSelectedManufacturer(manufacturer);
    resetSelectionsFrom('board'); // Reset board, image, device
    setActiveModal('none');
  }

  function handleBoardSelect(board: BoardInfo) {
    setSelectedBoard(board);
    resetSelectionsFrom('image'); // Reset image, device
    setActiveModal('none');
  }

  function handleImageSelect(image: ImageInfo) {
    setSelectedImage(image);
    resetSelectionsFrom('device'); // Reset device
    setActiveModal('none');
  }

  function handleDeviceSelect(device: BlockDevice) {
    setSelectedDevice(device);
    setActiveModal('none');
    // Start flashing immediately after device selection
    setIsFlashing(true);
  }

  async function handleCustomImage() {
    try {
      const result = await selectCustomImage();
      if (result) {
        // Detect board from filename
        let detectedBoard: BoardInfo | null = null;
        try {
          detectedBoard = await detectBoardFromFilename(result.name);
          if (detectedBoard) {
            logInfo('app', `Detected board from filename: ${detectedBoard.name} (${detectedBoard.slug})`);
          }
        } catch (err) {
          // Ignore detection errors, fall back to generic
          logWarn('app', `Failed to detect board from filename: ${err}`);
        }

        // Create a custom ImageInfo object
        const customImage: ImageInfo = {
          armbian_version: 'Custom',
          distro_release: result.name,
          kernel_branch: '',
          kernel_version: '',
          image_variant: 'custom',
          preinstalled_application: '',
          promoted: false,
          file_url: '',
          file_url_sha: null,
          file_size: result.size,
          download_repository: 'local',
          is_custom: true,
          custom_path: result.path,
        };

        // Reset selections and set board for display
        resetSelectionsFrom('manufacturer');

        // Use detected board if found, otherwise use generic custom board
        const displayBoard = detectedBoard || {
          slug: 'custom',
          name: t('custom.customImage'),
          vendor: 'custom',
          vendor_name: 'Custom',
          vendor_logo: null,
          image_count: 1,
          has_standard_support: false,
          has_community_support: false,
          has_platinum_support: false,
          has_eos_support: false,
          has_tvb_support: false,
          has_wip_support: false,
        };

        setSelectedBoard(displayBoard);
        setSelectedImage(customImage);
      }
    } catch (err) {
      console.error('Failed to select custom image:', err);
    }
  }

  function handleComplete() {
    setIsFlashing(false);
    resetSelectionsFrom('manufacturer'); // Reset all selections
  }

  function handleBackFromFlash() {
    setIsFlashing(false);
    setSelectedDevice(null); // Reset device to allow re-selection
  }

  function handleReset() {
    resetSelectionsFrom('manufacturer');
  }

  function handleNavigateToStep(step: SelectionStep) {
    // Reset selections from this step onwards, then open the modal
    resetSelectionsFrom(step);
    setActiveModal(step);
  }

  /**
   * Handle Armbian modal confirm - auto-select the detected board
   * Uses cached board to avoid redundant API fetch
   */
  const handleArmbianConfirm = useCallback(async () => {
    if (!detectedBoard) {
      logWarn('app', 'No detected board available for auto-selection');
      setShowArmbianModal(false);
      return;
    }

    await autoSelectBoard(detectedBoard);
    setShowArmbianModal(false);
  }, [detectedBoard, autoSelectBoard]);

  /**
   * Handle Armbian modal cancel - dismiss and proceed with manual selection
   */
  const handleArmbianCancel = useCallback(() => {
    logInfo('app', 'User cancelled Armbian board auto-selection');
    setShowArmbianModal(false);
  }, []);

  /**
   * Handle detection disabled from Armbian modal cancel - show informative toast
   */
  const handleDetectionDisabled = useCallback(() => {
    showError(t('armbian.disabledToast'));
  }, [showError, t]);

  return (
    <div className="app">
      <Header
        selectedManufacturer={selectedManufacturer}
        selectedBoard={selectedBoard}
        selectedImage={selectedImage}
        selectedDevice={selectedDevice}
        onReset={handleReset}
        onNavigateToStep={handleNavigateToStep}
        isFlashing={isFlashing}
      />

      <main className="main-content">
        {!isFlashing ? (
          <HomePage
            selectedManufacturer={selectedManufacturer}
            selectedBoard={selectedBoard}
            selectedImage={selectedImage}
            selectedDevice={selectedDevice}
            onChooseManufacturer={() => setActiveModal('manufacturer')}
            onChooseBoard={() => setActiveModal('board')}
            onChooseImage={() => setActiveModal('image')}
            onChooseDevice={() => setActiveModal('device')}
            onChooseCustomImage={handleCustomImage}
          />
        ) : (
          selectedBoard && selectedImage && selectedDevice && (
            <FlashProgress
              board={selectedBoard}
              image={selectedImage}
              device={selectedDevice}
              onComplete={handleComplete}
              onBack={handleBackFromFlash}
            />
          )
        )}
      </main>

      {/* Modals */}
      <ManufacturerModal
        isOpen={activeModal === 'manufacturer'}
        onClose={() => setActiveModal('none')}
        onSelect={handleManufacturerSelect}
      />

      <BoardModal
        isOpen={activeModal === 'board'}
        onClose={() => setActiveModal('none')}
        onSelect={handleBoardSelect}
        manufacturer={selectedManufacturer}
      />

      <ImageModal
        isOpen={activeModal === 'image'}
        onClose={() => setActiveModal('none')}
        onSelect={handleImageSelect}
        board={selectedBoard}
      />

      <DeviceModal
        isOpen={activeModal === 'device'}
        onClose={() => setActiveModal('none')}
        onSelect={handleDeviceSelect}
      />

      {/* Armbian board detection modal */}
      {armbianInfo && (
        <ArmbianBoardModal
          isOpen={showArmbianModal}
          onClose={handleArmbianCancel}
          onConfirm={handleArmbianConfirm}
          onDetectionDisabled={handleDetectionDisabled}
          armbianInfo={armbianInfo}
          boardInfo={detectedBoard}
          boardImageUrl={armbianBoardImageUrl}
        />
      )}

      {!isFlashing && <SettingsButton />}
    </div>
  );
}

export default App;
