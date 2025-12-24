import { type ReactNode, useEffect, useCallback, useState, useRef } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  searchBar?: ReactNode;
}

export function Modal({ isOpen, onClose, title, children, searchBar }: ModalProps) {
  const [isExiting, setIsExiting] = useState(false);
  const isExitingRef = useRef(false);

  const handleClose = useCallback(() => {
    if (isExitingRef.current) return;
    isExitingRef.current = true;
    setIsExiting(true);
    setTimeout(() => {
      setIsExiting(false);
      isExitingRef.current = false;
      onClose();
    }, 200); // Match the CSS exit animation duration
  }, [onClose]);

  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      handleClose();
    }
  }, [handleClose]);

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
          <h2 className="modal-title">{title}</h2>
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
