import { Factory, Cpu, Database, HardDrive, FolderOpen, Archive } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BoardInfo, ImageInfo, BlockDevice, Manufacturer } from '../../types';
import { MarqueeText } from '../shared';

interface HomePageProps {
  selectedManufacturer: Manufacturer | null;
  selectedBoard: BoardInfo | null;
  selectedImage: ImageInfo | null;
  selectedDevice: BlockDevice | null;
  onChooseManufacturer: () => void;
  onChooseBoard: () => void;
  onChooseImage: () => void;
  onChooseDevice: () => void;
  onChooseCustomImage: () => void;
  onOpenCacheManager: () => void;
  isOnline?: boolean;
}

/**
 * Main selection page with context-aware layouts:
 * - Offline: custom image + cached images buttons only
 * - Generic custom image (no board detected): OS + Storage buttons
 * - Detected custom/cached Armbian image: 4 read-only info buttons + change image
 * - Normal flow: 4 interactive selection buttons (Manufacturer > Board > OS > Storage)
 */
export function HomePage({
  selectedManufacturer,
  selectedBoard,
  selectedImage,
  selectedDevice,
  onChooseManufacturer,
  onChooseBoard,
  onChooseImage,
  onChooseDevice,
  onChooseCustomImage,
  onOpenCacheManager,
  isOnline = true,
}: HomePageProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;

  // Custom/cached images with a detected Armbian board show all 4 buttons.
  // Only truly generic images (non-Armbian .img files) show the 2-button layout.
  const hasDetectedBoard = selectedBoard && selectedBoard.slug !== 'custom' && selectedBoard.slug !== 'cached';
  const isGenericCustom = isCustomImage && !hasDetectedBoard;

  // Show offline layout when offline and no image has been selected yet
  const showOfflineLayout = !isOnline && !selectedManufacturer;

  if (showOfflineLayout) {
    return (
      <div className="home-page">
        <div className="home-offline-section">
          <div className="home-offline-actions">
            <div className="home-button-group">
              <button
                className="home-button"
                onClick={onChooseCustomImage}
              >
                <FolderOpen size={28} />
                <span className="home-button-text">{t('home.useCustomImage')}</span>
              </button>
            </div>

            <div className="home-button-group">
              <button
                className="home-button"
                onClick={onOpenCacheManager}
              >
                <Archive size={28} />
                <span className="home-button-text">{t('home.cachedImages')}</span>
              </button>
            </div>
          </div>

          <p className="home-offline-hint">{t('home.offlineHint')}</p>
        </div>
      </div>
    );
  }

  // Generic custom image (no detected board): show only OS + Storage
  if (isGenericCustom) {
    return (
      <div className="home-page">
        <div className="home-buttons-inline">
          <div className="home-button-group">
            <span className="home-button-label">{t('home.operatingSystem')}</span>
            <button
              className="home-button selected"
              onClick={onChooseCustomImage}
            >
              <Database size={28} />
              <span className="home-button-text-multi">
                <MarqueeText text={selectedImage.preinstalled_application || selectedImage.image_variant || ''} className="home-button-title" />
                <MarqueeText
                  text={selectedImage.distro_release && selectedImage.kernel_branch
                    ? `${selectedImage.distro_release} · ${selectedImage.kernel_branch}`
                    : selectedImage.distro_release || selectedImage.kernel_branch || '\u00A0'}
                  className="home-button-subtitle"
                />
              </span>
            </button>
          </div>

          <div className="home-button-group">
            <span className="home-button-label">{t('home.storage')}</span>
            <button
              className={`home-button ${selectedDevice ? 'selected' : ''}`}
              onClick={onChooseDevice}
              disabled={!selectedImage}
            >
              <HardDrive size={28} />
              {selectedDevice ? (
                <span className="home-button-text-multi">
                  <MarqueeText text={selectedDevice.name} className="home-button-title" />
                  <span className="home-button-subtitle">{selectedDevice.size_formatted}</span>
                </span>
              ) : (
                <span className="home-button-text">{t('home.chooseStorage')}</span>
              )}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Custom/cached Armbian image with detected board: 4 read-only buttons + change image
  if (isCustomImage && hasDetectedBoard) {
    return (
      <div className="home-page">
        <div className="home-buttons-inline">
          <div className="home-button-group">
            <span className="home-button-label">{t('home.manufacturer')}</span>
            <button className="home-button selected" disabled>
              <Factory size={28} />
              <span className="home-button-text-multi">
                <MarqueeText text={selectedManufacturer?.name || ''} className="home-button-title" />
                <span className="home-button-subtitle">&nbsp;</span>
              </span>
            </button>
          </div>

          <div className="home-button-group">
            <span className="home-button-label">{t('home.board')}</span>
            <button className="home-button selected" disabled>
              <Cpu size={28} />
              <span className="home-button-text-multi">
                <MarqueeText text={selectedBoard.name} className="home-button-title" />
                <span className="home-button-subtitle">{t('home.imageCount', { count: selectedBoard.image_count })}</span>
              </span>
            </button>
          </div>

          <div className="home-button-group">
            <span className="home-button-label">{t('home.operatingSystem')}</span>
            <button className="home-button selected" disabled>
              <Database size={28} />
              <span className="home-button-text-multi">
                <MarqueeText text={selectedImage.preinstalled_application || selectedImage.image_variant || ''} className="home-button-title" />
                <MarqueeText
                  text={selectedImage.distro_release && selectedImage.kernel_branch
                    ? `${selectedImage.distro_release} · ${selectedImage.kernel_branch}`
                    : selectedImage.distro_release || selectedImage.kernel_branch || '\u00A0'}
                  className="home-button-subtitle"
                />
              </span>
            </button>
          </div>

          <div className="home-button-group">
            <span className="home-button-label">{t('home.storage')}</span>
            <button
              className={`home-button ${selectedDevice ? 'selected' : ''}`}
              onClick={onChooseDevice}
            >
              <HardDrive size={28} />
              {selectedDevice ? (
                <span className="home-button-text-multi">
                  <MarqueeText text={selectedDevice.name} className="home-button-title" />
                  <span className="home-button-subtitle">{selectedDevice.size_formatted}</span>
                </span>
              ) : (
                <span className="home-button-text">{t('home.chooseStorage')}</span>
              )}
            </button>
          </div>
        </div>

        <div className="home-custom-section">
          <button
            className="home-custom-button"
            onClick={selectedImage.image_variant === 'cached' ? onOpenCacheManager : onChooseCustomImage}
          >
            <FolderOpen size={16} />
            {selectedImage.image_variant === 'cached'
              ? t('home.changeCachedImage')
              : t('home.changeCustomImage')}
          </button>
        </div>
      </div>
    );
  }

  // Normal flow: all 4 buttons
  return (
    <div className="home-page">
      <div className="home-buttons-inline">
        <div className="home-button-group">
          <span className="home-button-label">{t('home.manufacturer')}</span>
          <button
            className={`home-button ${selectedManufacturer ? 'selected' : ''}`}
            onClick={onChooseManufacturer}
          >
            <Factory size={28} />
            {selectedManufacturer ? (
              <span className="home-button-text-multi">
                <MarqueeText text={selectedManufacturer.name} className="home-button-title" />
                <span className="home-button-subtitle">&nbsp;</span>
              </span>
            ) : (
              <span className="home-button-text">{t('home.chooseBrand')}</span>
            )}
          </button>
        </div>

        <div className="home-button-group">
          <span className="home-button-label">{t('home.board')}</span>
          <button
            className={`home-button ${selectedBoard ? 'selected' : ''}`}
            onClick={onChooseBoard}
            disabled={!selectedManufacturer}
          >
            <Cpu size={28} />
            {selectedBoard ? (
              <span className="home-button-text-multi">
                <MarqueeText text={selectedBoard.name} className="home-button-title" />
                <span className="home-button-subtitle">{t('home.imageCount', { count: selectedBoard.image_count })}</span>
              </span>
            ) : (
              <span className="home-button-text">{t('home.chooseBoard')}</span>
            )}
          </button>
        </div>

        <div className="home-button-group">
          <span className="home-button-label">{t('home.operatingSystem')}</span>
          <button
            className={`home-button ${selectedImage ? 'selected' : ''}`}
            onClick={onChooseImage}
            disabled={!selectedBoard}
          >
            <Database size={28} />
            {selectedImage ? (
              <span className="home-button-text-multi">
                <MarqueeText text={selectedImage.preinstalled_application || selectedImage.image_variant || ''} className="home-button-title" />
                <MarqueeText
                  text={selectedImage.distro_release && selectedImage.kernel_branch
                    ? `${selectedImage.distro_release} · ${selectedImage.kernel_branch}`
                    : selectedImage.distro_release || selectedImage.kernel_branch || '\u00A0'}
                  className="home-button-subtitle"
                />
              </span>
            ) : (
              <span className="home-button-text">{t('home.chooseOs')}</span>
            )}
          </button>
        </div>

        <div className="home-button-group">
          <span className="home-button-label">{t('home.storage')}</span>
          <button
            className={`home-button ${selectedDevice ? 'selected' : ''}`}
            onClick={onChooseDevice}
            disabled={!selectedImage}
          >
            <HardDrive size={28} />
            {selectedDevice ? (
              <span className="home-button-text-multi">
                <MarqueeText text={selectedDevice.name} className="home-button-title" />
                <span className="home-button-subtitle">{selectedDevice.size_formatted}</span>
              </span>
            ) : (
              <span className="home-button-text">{t('home.chooseStorage')}</span>
            )}
          </button>
        </div>
      </div>

      {!selectedManufacturer && (
        <div className="home-custom-section">
          <button
            className="home-custom-button"
            onClick={onChooseCustomImage}
          >
            <FolderOpen size={16} />
            {t('home.useCustomImage')}
          </button>
        </div>
      )}
    </div>
  );
}
