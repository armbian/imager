import { useState, useEffect } from 'react';
import { HardDrive, Usb, Disc, FileImage } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BoardInfo, ImageInfo, BlockDevice, AutoconfigConfig } from '../../types';
import { isEdlImage } from '../../types';
import { getOsName } from '../../assets/os-logos';
import { getMonoLogo } from '../../config/mono-logos';
import { distroBlock } from '../../utils/distroTheme';
import { formatImageIdentity } from '../../utils';
import { getCachedBoardImage } from '../../hooks/useTauri';
import { useFlashOperation } from '../../hooks/useFlashOperation';
import { FlashStageIcon, getStageKey, isIndeterminateStage } from './FlashStageIcon';
import { FlashActions } from './FlashActions';
import { FlashPhaseDots } from './FlashPhaseDots';
import { ErrorDisplay, MarqueeText, ConfirmationDialog, BoardImage } from '../shared';

interface FlashProgressProps {
  board: BoardInfo;
  image: ImageInfo;
  device: BlockDevice;
  /** Opt-in autoconfig profile config to write on first boot; null when none selected. */
  autoconfig?: AutoconfigConfig | null;
  onComplete: () => void;
  onBack: () => void;
}

export function FlashProgress({
  board,
  image,
  device,
  autoconfig,
  onComplete,
  onBack,
}: FlashProgressProps) {
  const { t } = useTranslation();
  const [boardImageUrl, setBoardImageUrl] = useState<string | null>(null);

  const {
    stage,
    phases,
    progress,
    error,
    showShaWarning,
    handleCancel,
    handleRetry,
    handleBack,
    handleShaWarningConfirm,
    handleShaWarningCancel,
  } = useFlashOperation({ image, device, soc: board.soc, boardSlug: board.slug, autoconfig, onBack });

  useEffect(() => {
    getCachedBoardImage(board.slug)
      .then(setBoardImageUrl)
      .catch(() => {
        /* fall back to placeholder image */
      });
  }, [board.slug]);

  /** Branded OS identity, matching the sidebar/OS card (e.g. "Armbian 26.2.0 GNOME"). */
  function getImageDisplayText(): string {
    return formatImageIdentity(image, t).title;
  }

  // Stages with a breathing bar instead of a percentage.
  const isIndeterminate = isIndeterminateStage(stage);

  const showHeader = stage !== 'authorizing' && stage !== 'error';
  const isError = stage === 'error';
  const isComplete = stage === 'complete';
  const isCustomIcon = image.is_custom && board.slug === 'custom';
  const isEdl = isEdlImage(image);

  // Glow brightness tracks progress; indeterminate stages sit at mid-glow.
  const glowProgress = isComplete ? 100 : isIndeterminate ? 50 : progress;

  return (
    <div className={`flash-container ${!showHeader ? 'centered' : ''}`}>
      {showHeader ? (
        <div className="flash-stage">
          <div
            className="flash-stage__board"
            style={{ '--flash-progress': `${glowProgress}` } as React.CSSProperties}
          >
            {isCustomIcon ? (
              // No board photo: a dedicated custom-image glyph floating over the glow,
              // framed by breathing rings — always the same, regardless of the file.
              <div className="flash-custom-glyph" aria-hidden="true">
                <span className="flash-custom-glyph__ring" />
                <span className="flash-custom-glyph__ring" />
                <FileImage className="flash-custom-glyph__icon" strokeWidth={1.5} />
              </div>
            ) : (
              <BoardImage src={boardImageUrl} alt={board.name} className="flash-board-image" />
            )}
          </div>

          <div className="flash-stage__main">
            <div className="flash-info">
              <h2>{board.name}</h2>
              <div className="flash-info-badges">
                <div
                  className="os-badge"
                  title={image.is_custom ? image.distro_release : undefined}
                >
                  {(() => {
                    // White vector mark on a distro-tinted chip (Ubuntu/Debian), matching the OS gallery.
                    const logo = getMonoLogo(
                      image.distro_release,
                      image.preinstalled_application
                    );
                    return logo ? (
                      <span
                        className="os-badge-chip"
                        style={{ background: distroBlock(getOsName(image.distro_release)) }}
                      >
                        <img
                          src={logo}
                          alt={getOsName(image.distro_release)}
                          className="os-badge-chip__logo"
                        />
                      </span>
                    ) : (
                      <Disc size={20} className="os-badge-icon" />
                    );
                  })()}
                  <MarqueeText text={getImageDisplayText()} className="os-badge-text" />
                </div>
                <div className="flash-device-row">
                  {isEdl ? <Usb size={16} /> : <HardDrive size={16} />}
                  <MarqueeText text={device.model || device.name} className="flash-device-name" />
                  {device.size_formatted && <span className="flash-device-size">{device.size_formatted}</span>}
                </div>
              </div>
            </div>

            <div className="flash-progress">
              <div
                className={`flash-progress__head${isComplete ? ' is-done' : ''}`}
                key={stage}
              >
                <FlashStageIcon stage={stage} />
                <span className={`flash-progress__label${isComplete ? ' is-done' : ''}`}>
                  {t(getStageKey(stage))}
                </span>
              </div>

              {!isComplete && (
                <div className="flash-progress__bar">
                  <div className="flash-track" role="presentation" aria-hidden="true">
                    <div
                      className={`flash-track__fill${isIndeterminate ? ' is-indeterminate' : ''}`}
                      style={isIndeterminate ? undefined : { width: `${progress}%` }}
                    />
                  </div>
                  <FlashPhaseDots stage={stage} phases={phases} />
                </div>
              )}

              {isComplete && (
                <p className="flash-success-hint">
                  {isEdl
                    ? t('flash.successHintQdl')
                    : image.is_custom
                      ? t('flash.successHintCustom')
                      : t('flash.successHint', { boardName: board.name })}
                </p>
              )}
            </div>

            <FlashActions
              stage={stage}
              onComplete={onComplete}
              onBack={handleBack}
              onRetry={handleRetry}
              onCancel={handleCancel}
            />
          </div>
        </div>
      ) : isError ? (
        // Failure: full-screen two-column error layout (hero + diagnosis + remedy).
        <ErrorDisplay
          error={error || ''}
          onRetry={handleRetry}
          onCancel={handleBack}
        />
      ) : (
        // Authorizing (pkexec): narrow centered status column.
        <div className="flash-fallback">
          <div className="flash-status__head" key={stage}>
            <FlashStageIcon stage={stage} />
            <h3>{t(getStageKey(stage))}</h3>
          </div>

          <FlashActions
            stage={stage}
            onComplete={onComplete}
            onBack={handleBack}
            onRetry={handleRetry}
            onCancel={handleCancel}
          />
        </div>
      )}

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
