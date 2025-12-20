import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Header, HomePage } from './components/layout';
import { ManufacturerModal, BoardModal, ImageModal, DeviceModal } from './components/modals';
import { FlashProgress } from './components/flash';
import { AppVersion } from './components/shared';
import { selectCustomImage } from './hooks/useTauri';
import { useDeviceMonitor } from './hooks/useDeviceMonitor';
import type { BoardInfo, ImageInfo, BlockDevice, ModalType, SelectionStep, Manufacturer } from './types';
import './styles/index.css';

function App() {
  const { t } = useTranslation();
  const [isFlashing, setIsFlashing] = useState(false);
  const [activeModal, setActiveModal] = useState<ModalType>('none');
  const [selectedManufacturer, setSelectedManufacturer] = useState<Manufacturer | null>(null);
  const [selectedBoard, setSelectedBoard] = useState<BoardInfo | null>(null);
  const [selectedImage, setSelectedImage] = useState<ImageInfo | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<BlockDevice | null>(null);

  // Monitor selected device - clear if disconnected (only when not flashing)
  useDeviceMonitor(
    selectedDevice,
    useCallback(() => setSelectedDevice(null), []),
    !isFlashing
  );

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
        // Create a custom ImageInfo object
        const customImage: ImageInfo = {
          armbian_version: 'Custom',
          distro_release: result.name,
          kernel_branch: '',
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
        // Reset selections and set custom board/image for display purposes
        resetSelectionsFrom('manufacturer');
        setSelectedBoard({
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
        });
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

      <AppVersion />
    </div>
  );
}

export default App;
