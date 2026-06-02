// Modal shown on an Armbian host to confirm auto-selecting the detected board

import { useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { ImageOff } from 'lucide-react';
import { BoardBadges } from '../shared/BoardBadges';
import type { ArmbianReleaseInfo, BoardInfo } from '../../types';
import { setArmbianBoardDetection } from '../../hooks/useSettings';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';

interface ArmbianBoardModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  onDetectionDisabled?: () => void;
  armbianInfo: ArmbianReleaseInfo;
  boardInfo?: BoardInfo | null;
  boardImageUrl?: string | null;
}

export function ArmbianBoardModal({
  isOpen,
  onClose,
  onConfirm,
  onDetectionDisabled,
  armbianInfo,
  boardInfo,
  boardImageUrl,
}: ArmbianBoardModalProps) {
  const { t } = useTranslation();

  const { isExiting, handleClose, handleAction } = useModalExitAnimation({
    onClose,
    duration: 200,
    onExiting: () => {
      // 'auto' enables silent auto-selection on future runs
      setArmbianBoardDetection('auto');
    },
  });

  const handleConfirm = useCallback(() => {
    handleAction(() => {
      onConfirm();
    });
  }, [handleAction, onConfirm]);

  const handleCloseWithCallback = useCallback(() => {
    handleAction(() => {
      setArmbianBoardDetection('disabled');
      onDetectionDisabled?.();
    });
  }, [handleAction, onDetectionDisabled]);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen && !isExiting) {
        handleCloseWithCallback();
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
    };
  }, [isOpen, isExiting, handleCloseWithCallback]);

  if (!isOpen) return null;

  const animationClass = isExiting ? 'modal-exiting' : 'modal-entering';

  return (
    <div className={`modal-overlay ${animationClass}`} onClick={handleClose}>
      <div
        className={`modal modal-compact armbian-modal ${animationClass}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="armbian-board-name"
        onClick={(e) => e.stopPropagation()}
      >
        <button className="armbian-board-close" onClick={handleClose} aria-label="Close">
          ✕
        </button>

        <div className="modal-body armbian-board-modal">
          <div className="armbian-board-hero">
            <div className="armbian-board-image">
              {boardImageUrl ? (
                <img src={boardImageUrl} alt={armbianInfo.board_name} />
              ) : (
                <div className="board-image-placeholder">
                  <ImageOff size={40} />
                </div>
              )}
            </div>
          </div>

          <h3 id="armbian-board-name" className="armbian-board-name">{armbianInfo.board_name}</h3>

          {boardInfo && <BoardBadges board={boardInfo} className="centered" />}

          <p className="armbian-board-description">{t('armbian.description')}</p>

          <div className="armbian-board-actions">
            <button className="btn btn-secondary" onClick={handleCloseWithCallback} disabled={isExiting}>
              {t('common.cancel')}
            </button>
            <button className="btn btn-primary" onClick={handleConfirm} disabled={isExiting}>
              {t('common.confirm')}
            </button>
          </div>
          <p className="armbian-board-hint">{t('armbian.cancelHint')}</p>
        </div>
      </div>
    </div>
  );
}
