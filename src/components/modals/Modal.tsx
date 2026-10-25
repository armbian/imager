// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { type ReactNode, useEffect, useId } from 'react';
import { createPortal } from 'react-dom';
import { X, ChevronLeft } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDialogFocus } from '../../hooks/useDialogFocus';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  searchBar?: ReactNode;
  footer?: (close: () => void) => ReactNode;
  className?: string;
  showBack?: boolean;
  onBack?: () => void;
}

export function Modal({ isOpen, ...props }: ModalProps) {
  const { isExiting, handleClose } = useModalExitAnimation({ onClose: props.onClose });

  useEffect(() => {
    if (!isOpen) return;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = '';
    };
  }, [isOpen]);

  if (!isOpen && !isExiting) return null;

  return createPortal(<ModalFrame {...props} isExiting={isExiting} onDismiss={handleClose} />, document.body);
}

type ModalFrameProps = Omit<ModalProps, 'isOpen'> & { isExiting: boolean; onDismiss: () => void };

// Mounted only while visible, so useDialogFocus takes focus on open and hands it back on close.
function ModalFrame({ title, children, searchBar, footer, className, showBack, onBack, isExiting, onDismiss }: ModalFrameProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const dialogRef = useDialogFocus<HTMLDivElement>({ onEscape: showBack && onBack ? onBack : onDismiss });
  const animationClass = isExiting ? 'modal-exiting' : 'modal-entering';

  return (
    <div className={`modal-overlay ${animationClass}`} onClick={onDismiss}>
      <div
        ref={dialogRef}
        className={`modal ${animationClass}${className ? ` ${className}` : ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div className="modal-header-left">
            {showBack && onBack && (
              <button type="button" className="modal-back" onClick={onBack} aria-label={t('common.back')}>
                <ChevronLeft size={20} />
              </button>
            )}
            <h2 id={titleId} className="modal-title">{title}</h2>
          </div>
          <button type="button" className="modal-close" onClick={onDismiss} aria-label={t('common.close')}>
            <X size={20} />
          </button>
        </div>
        {searchBar}
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-footer">{footer(onDismiss)}</div>}
      </div>
    </div>
  );
}
