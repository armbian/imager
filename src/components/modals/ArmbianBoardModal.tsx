/**
 * Armbian Board Detection Modal
 *
 * Shows when the app detects it's running on an Armbian system.
 * Allows the user to confirm auto-selection of the detected board
 * or cancel and proceed with manual selection.
 */

import { useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
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
  boardImageUrl?: string | null; // Preloaded image URL from parent
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
      // Set to 'auto' on confirm (silent auto-selection in future), 'disabled' on cancel
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
      // Set to 'disabled' when user cancels
      setArmbianBoardDetection('disabled');
      onDetectionDisabled?.();
    });
  }, [handleAction, onDetectionDisabled]);

  /**
   * Handle Escape key press
   */
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
      <div className={`modal modal-compact ${animationClass}`} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-header-left">
            <h2 className="modal-title">{t('armbian.title')}</h2>
          </div>
          <button className="modal-close" onClick={handleClose} aria-label="Close">
            ✕
          </button>
        </div>

        <div className="modal-body armbian-board-modal">
          {/* Board image with accent glow */}
          <div className="armbian-board-hero">
            <div className="armbian-board-image">
              {boardImageUrl ? (
                <img src={boardImageUrl} alt={armbianInfo.board_name} />
              ) : (
                <div className="board-image-placeholder">
                  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="2" y="3" width="20" height="14" rx="2" />
                    <path d="M8 21h8" />
                    <path d="M12 17v4" />
                    <circle cx="12" cy="10" r="2" />
                  </svg>
                </div>
              )}
            </div>
          </div>

          {/* Board name */}
          <h3 className="armbian-board-name">{armbianInfo.board_name}</h3>

          {/* Support badges */}
          {boardInfo && <BoardBadges board={boardInfo} className="centered" />}

          {/* Description */}
          <p className="armbian-board-description">{t('armbian.description')}</p>

          {/* Action buttons */}
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
