import { useState, useEffect } from 'react';
import { HardDrive, Disc, FileImage } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BoardInfo, ImageInfo, BlockDevice } from '../../types';
import { getImageLogo, getOsName } from '../../assets/os-logos';
import { getBoardImageUrl } from '../../hooks/useTauri';
import { useFlashOperation } from '../../hooks/useFlashOperation';
import { FlashStageIcon, getStageKey } from './FlashStageIcon';
import { FlashActions } from './FlashActions';
import { ErrorDisplay, MarqueeText, ConfirmationDialog } from '../shared';
import fallbackImage from '../../assets/armbian-logo_nofound.png';

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
  const [boardImageUrl, setBoardImageUrl] = useState<string | null>(null);
  const [imageLoadError, setImageLoadError] = useState(false);

  // All flash operation logic is encapsulated in this hook
  const {
    stage,
    progress,
    error,
    showShaWarning,
    handleCancel,
    handleRetry,
    handleBack,
    handleShaWarningConfirm,
    handleShaWarningCancel,
  } = useFlashOperation({ image, device, onBack });

  // Load board image for header display
  useEffect(() => {
    getBoardImageUrl(board.slug)
      .then(setBoardImageUrl)
      .catch(() => { /* Ignore */ });
  }, [board.slug]);

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
          {image.is_custom && board.slug === 'custom' ? (
            // Generic icon for non-Armbian or undetected custom images
            <div className="flash-board-image flash-custom-image-icon">
              <FileImage size={40} />
            </div>
          ) : (
            // Board image for detected Armbian custom images OR standard images
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

      {showShaWarning && (
        <ConfirmationDialog
          isOpen={showShaWarning}
          title={t('flash.noShaTitle')}
          message={t('flash.noShaMessage')}
          confirmText={t('common.confirm')}
          isDanger={false}
          onCancel={handleShaWarningCancel}
          onConfirm={handleShaWarningConfirm}
        />
      )}
    </div>
  );
}
