import { useState, useMemo, useEffect } from 'react';
import { Download, Package, Monitor, Terminal, Zap, Star, Layers, Shield, FlaskConical, AppWindow, Box } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay, ListItemSkeleton, ConfirmationDialog } from '../shared';
import type { BoardInfo, ImageInfo, ImageFilterType } from '../../types';
import { getImagesForBoard } from '../../hooks/useTauri';
import { useAsyncDataWhen } from '../../hooks/useAsyncData';
import {
  getOsInfo,
  getAppInfo,
  getDesktopEnv,
  getKernelType,
  DESKTOP_BADGES,
  KERNEL_BADGES,
  DESKTOP_ENVIRONMENTS,
  UI,
} from '../../config';
import { formatFileSize, DEFAULT_COLOR } from '../../utils';

interface ImageModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (image: ImageInfo) => void;
  board: BoardInfo | null;
}

/**
 * Filter predicates - defined once, used for both checking availability and filtering
 * Each predicate returns true if the image matches the filter criteria
 */
const IMAGE_FILTER_PREDICATES: Record<Exclude<ImageFilterType, 'all'>, (img: ImageInfo) => boolean> = {
  // Recommended: promoted images
  recommended: (img) => img.promoted === true,
  // Stable: from archive repository and not trunk version
  stable: (img) => img.download_repository === 'archive' && !img.armbian_version.includes('trunk'),
  // Nightly: trunk versions
  nightly: (img) => img.armbian_version.includes('trunk'),
  // Apps: has preinstalled application
  apps: (img) => !!(img.preinstalled_application && img.preinstalled_application.length > 0),
  // Barebone/Minimal: no desktop environment and no preinstalled apps
  barebone: (img) => {
    const variant = img.image_variant.toLowerCase();
    const hasDesktop = DESKTOP_ENVIRONMENTS.some(de => variant.includes(de));
    const hasApp = img.preinstalled_application && img.preinstalled_application.length > 0;
    return !hasDesktop && !hasApp;
  },
};

/** Check if any images match a filter */
function hasImagesForFilter(images: ImageInfo[], filter: Exclude<ImageFilterType, 'all'>): boolean {
  return images.some(IMAGE_FILTER_PREDICATES[filter]);
}

/** Apply filter to images */
function applyFilter(images: ImageInfo[], filter: ImageFilterType): ImageInfo[] {
  if (filter === 'all') return images;
  return images.filter(IMAGE_FILTER_PREDICATES[filter]);
}

/** Filter button configuration for data-driven rendering */
const FILTER_BUTTONS: Array<{
  key: Exclude<ImageFilterType, 'all'>;
  labelKey: string;
  icon: typeof Star;
}> = [
  { key: 'recommended', labelKey: 'modal.promoted', icon: Star },
  { key: 'stable', labelKey: 'modal.stable', icon: Shield },
  { key: 'nightly', labelKey: 'modal.nightly', icon: FlaskConical },
  { key: 'apps', labelKey: 'modal.apps', icon: AppWindow },
  { key: 'barebone', labelKey: 'modal.minimal', icon: Box },
];

export function ImageModal({ isOpen, onClose, onSelect, board }: ImageModalProps) {
  const { t } = useTranslation();
  const [filterType, setFilterType] = useState<ImageFilterType>('all');
  const [showSkeleton, setShowSkeleton] = useState(false);
  // State for unstable image warning
  const [pendingImage, setPendingImage] = useState<ImageInfo | null>(null);
  const [showUnstableWarning, setShowUnstableWarning] = useState(false);

  // Use hook for async data fetching
  const { data: allImages, loading, error, reload } = useAsyncDataWhen<ImageInfo[]>(
    isOpen && !!board,
    () => getImagesForBoard(board!.slug, undefined, undefined, undefined, false),
    [isOpen, board?.slug]
  );

  // Derive images ready state
  const imagesReady = useMemo(() => {
    return allImages && allImages.length > 0;
  }, [allImages]);

  // Show skeleton with minimum delay
  useEffect(() => {
    let skeletonTimeout: NodeJS.Timeout;

    if (loading) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Show skeleton during loading
      setShowSkeleton(true);
    } else if (imagesReady) {
      // Keep skeleton visible for at least 300ms
      skeletonTimeout = setTimeout(() => {
        setShowSkeleton(false);
      }, 300);
    }

    return () => {
      if (skeletonTimeout) {
        clearTimeout(skeletonTimeout);
      }
    };
  }, [loading, imagesReady]);

  // Reset warning state when modal is closed
  useEffect(() => {
    if (!isOpen) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Reset warning state when modal closes to prevent stale state from leaking into next session
      setPendingImage(null);
      setShowUnstableWarning(false);
    }
  }, [isOpen]);

  /**
   * Handle image click - show warning for unstable images before selecting
   */
  function handleImageClick(image: ImageInfo) {
    // Check if warning is needed
    const isNightly = image.armbian_version.includes('trunk');
    const isCommunityBoard = board?.has_community_support === true;

    // No warning for custom images or stable images on supported boards
    if (!isNightly && !isCommunityBoard) {
      onSelect(image);
      return;
    }

    // Show warning for nightly builds or community-supported boards
    setPendingImage(image);
    setShowUnstableWarning(true);
  }

  /**
   * Confirm unstable image selection - proceed with pending image
   */
  function handleUnstableWarningConfirm() {
    if (pendingImage) {
      onSelect(pendingImage);
      setPendingImage(null);
    }
    setShowUnstableWarning(false);
  }

  /**
   * Cancel unstable image selection - return to image list
   */
  function handleUnstableWarningCancel() {
    setPendingImage(null);
    setShowUnstableWarning(false);
  }

  // Calculate available filters based on all images
  const availableFilters = useMemo(() => {
    if (!allImages) return { recommended: false, stable: false, nightly: false, apps: false, barebone: false };
    return {
      recommended: hasImagesForFilter(allImages, 'recommended'),
      stable: hasImagesForFilter(allImages, 'stable'),
      nightly: hasImagesForFilter(allImages, 'nightly'),
      apps: hasImagesForFilter(allImages, 'apps'),
      barebone: hasImagesForFilter(allImages, 'barebone'),
    };
  }, [allImages]);

  // Apply filter using useMemo
  const filteredImages = useMemo(() => {
    if (!allImages) return [];
    return applyFilter(allImages, filterType);
  }, [allImages, filterType]);

  const title = t('modal.selectImage');

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title}>
      <div className="modal-filter-bar">
        <button
          className={`filter-btn ${filterType === 'all' ? 'active' : ''}`}
          onClick={() => setFilterType('all')}
        >
          <Layers size={14} />
          {t('modal.allImages')}
        </button>
        {FILTER_BUTTONS.map(({ key, labelKey, icon: Icon }) =>
          availableFilters[key] && (
            <button
              key={key}
              className={`filter-btn ${filterType === key ? 'active' : ''}`}
              onClick={() => setFilterType(key)}
            >
              <Icon size={14} />
              {t(labelKey)}
            </button>
          )
        )}
      </div>

      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : (
        <>
          {showSkeleton && <ListItemSkeleton count={UI.SKELETON.IMAGE_MODAL} />}
          {filteredImages.length === 0 && !showSkeleton && (
            <div className="no-results">
              <Package size={48} />
              <p>{t('modal.noImages')}</p>
              <button onClick={() => setFilterType('all')} className="btn btn-secondary">
                {t('modal.allImages')}
              </button>
            </div>
          )}
          <div className="modal-list">
          {!showSkeleton && filteredImages.map((image, index) => {
            const desktopEnv = getDesktopEnv(image.image_variant);
            const kernelType = getKernelType(image.kernel_branch);
            const osInfo = getOsInfo(image.distro_release);
            const appInfo = getAppInfo(image.preinstalled_application);

            // Use app logo if available, otherwise use OS logo
            const displayInfo = appInfo || osInfo;

            return (
              <button
                key={index}
                className={`list-item ${image.promoted ? 'promoted' : ''}`}
                onClick={() => handleImageClick(image)}
              >
                {/* OS/App Icon */}
                <div className="list-item-icon os-icon" style={{ backgroundColor: displayInfo?.color || DEFAULT_COLOR }}>
                  {displayInfo?.logo ? (
                    <img src={displayInfo.logo} alt={displayInfo.name} />
                  ) : (
                    <Package size={32} color="white" />
                  )}
                </div>

                <div className="list-item-content" style={{ flex: 1 }}>
                  <div className="list-item-title">
                    Armbian {image.armbian_version} {image.distro_release}
                    {image.preinstalled_application && (
                      <span className="badge" style={{ marginLeft: 8, background: appInfo?.badgeColor || 'var(--accent)', color: 'white' }}>
                        {image.preinstalled_application}
                      </span>
                    )}
                  </div>
                  <div className="list-item-badges" style={{ display: 'flex', gap: 6, marginTop: 6, flexWrap: 'wrap' }}>
                    {image.promoted && (
                      <span className="badge badge-recommended">
                        <Star size={11} />
                        {t('modal.promoted')}
                      </span>
                    )}
                    {desktopEnv && DESKTOP_BADGES[desktopEnv] ? (
                      <span className="badge badge-desktop">
                        <Monitor size={11} />
                        {DESKTOP_BADGES[desktopEnv].label}
                      </span>
                    ) : (
                      <span className="badge badge-cli">
                        <Terminal size={11} />
                        CLI
                      </span>
                    )}
                    {kernelType && KERNEL_BADGES[kernelType] && (
                      <span className="badge badge-kernel">
                        <Zap size={11} />
                        {KERNEL_BADGES[kernelType].label}
                      </span>
                    )}
                  </div>
                </div>
                <span className="badge badge-size">
                  <Download size={11} />
                  {formatFileSize(image.file_size, t('common.unknown'))}
                </span>
              </button>
            );
          })}
          </div>
        </>
      )}

      {/* Image status warning dialog */}
      {showUnstableWarning && pendingImage && (
        <ConfirmationDialog
          isOpen={showUnstableWarning}
          title={t('modal.imageStatusTitle')}
          message={
            board?.has_community_support === true
              ? t('modal.communityBoardMessage')
              : t('modal.nightlyBuildMessage')
          }
          confirmText={t('common.confirm')}
          cancelText={t('common.cancel')}
          isDanger={false}
          onCancel={handleUnstableWarningCancel}
          onConfirm={handleUnstableWarningConfirm}
        />
      )}
    </Modal>
  );
}
