import { type ReactNode, useEffect, useCallback } from 'react';
import { X, ChevronLeft } from 'lucide-react';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  searchBar?: ReactNode;
  showBack?: boolean;
  onBack?: () => void;
}

export function Modal({ isOpen, onClose, title, children, searchBar, showBack, onBack }: ModalProps) {
  const { isExiting, handleClose } = useModalExitAnimation({
    onClose,
    duration: 200,
  });

  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      if (showBack && onBack) {
        onBack();
      } else {
        handleClose();
      }
    }
  }, [handleClose, showBack, onBack]);

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    } else {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = '';
    }
    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = '';
    };
  }, [isOpen, handleEscape]);

  if (!isOpen && !isExiting) return null;

  const animationClass = isExiting ? 'modal-exiting' : 'modal-entering';

  return (
    <div className={`modal-overlay ${animationClass}`} onClick={handleClose}>
      <div className={`modal ${animationClass}`} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-header-left">
            {showBack && onBack && (
              <button className="modal-back" onClick={onBack}>
                <ChevronLeft size={20} />
              </button>
            )}
            <h2 className="modal-title">{title}</h2>
          </div>
          <button className="modal-close" onClick={handleClose}>
            <X size={20} />
          </button>
        </div>
        {searchBar}
        <div className="modal-body">
          {children}
        </div>
      </div>
    </div>
  );
}
