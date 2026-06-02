// Reusable confirmation dialog

import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { TriangleAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ConfirmationDialogProps {
  /** Whether the dialog is visible */
  isOpen: boolean;
  /** Dialog title */
  title: string;
  /** Main message/description */
  message: string;
  /** Additional content to display (e.g., device info) */
  children?: ReactNode;
  /** Warning text shown at bottom */
  warning?: string;
  /** Cancel button text (defaults to 'Cancel') */
  cancelText?: string;
  /** Confirm button text (defaults to 'Confirm') */
  confirmText?: string;
  /** Whether confirm button is danger styled (default: true) */
  isDanger?: boolean;
  /** Called when dialog is cancelled/closed */
  onCancel: () => void;
  /** Called when action is confirmed */
  onConfirm: () => void;
}

/** Modal confirmation dialog with cancel/confirm actions. */
export function ConfirmationDialog({
  isOpen,
  title,
  message,
  children,
  warning,
  cancelText,
  confirmText,
  isDanger = true,
  onCancel,
  onConfirm,
}: ConfirmationDialogProps) {
  const { t } = useTranslation();

  if (!isOpen) return null;

  // Portal to <body> so the fixed overlay covers the whole viewport, escaping any
  // transformed ancestor (e.g. the animated settings shell) that would otherwise trap it.
  return createPortal(
    <div className="modal-overlay" onClick={onCancel}>
      <div
        className={`modal confirm-modal ${isDanger ? 'is-danger' : ''}`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Icon color comes from CSS (currentColor): amber for warnings, red for danger. */}
        <div className="confirm-icon">
          <TriangleAlert size={30} />
        </div>
        <h3 className="confirm-title">{title}</h3>
        <p className="confirm-text">{message}</p>
        {children}
        {warning && <p className="confirm-warning">{warning}</p>}
        <div className="confirm-actions">
          <button className="btn btn-secondary" onClick={onCancel}>
            {cancelText || t('common.cancel')}
          </button>
          <button
            className={`btn ${isDanger ? 'btn-danger' : 'btn-primary'}`}
            onClick={onConfirm}
          >
            {confirmText || t('common.confirm')}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
