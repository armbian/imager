// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { Upload, CircleAlert, CircleX, Loader2, QrCode, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLogShare } from '../../hooks/useLogShare';
import { UI } from '../../config';
import { LogShareModal } from './LogShareModal';
import { LogUploadRemedy } from './LogUploadRemedy';

interface ErrorDisplayProps {
  error: string;
  onRetry?: () => void;
  /** Dismiss/cancel handler; renders the secondary button in full-screen mode. */
  onCancel?: () => void;
  compact?: boolean;
}

function CompactErrorDisplay({ message, onRetry }: { message: string; onRetry?: () => void }) {
  const { t } = useTranslation();
  const { share, uploading, uploaded, error: uploadError, triggerRef, modalProps } = useLogShare();

  const shareLabel = uploading
    ? t('errorDisplay.uploadingLogs')
    : uploaded
      ? t('errorDisplay.showLogLink')
      : t('errorDisplay.uploadLogs');

  return (
    <div className="error-display-compact">
      <div className="error-display-message">
        <CircleAlert size={UI.ICON_SIZE.BUTTON} />
        <span>{message}</span>
      </div>
      <div className="error-display-actions">
        {onRetry && (
          <button type="button" onClick={onRetry} className="btn btn-primary btn-sm">
            {t('errorDisplay.retry')}
          </button>
        )}
        <button
          type="button"
          ref={triggerRef}
          className="btn btn-secondary btn-sm"
          onClick={share}
          disabled={uploading}
          aria-busy={uploading}
        >
          {uploading ? (
            <Loader2 size={UI.ICON_SIZE.UPLOAD_ERROR} className="spinning" />
          ) : uploaded ? (
            <QrCode size={UI.ICON_SIZE.UPLOAD_ERROR} />
          ) : (
            <Upload size={UI.ICON_SIZE.UPLOAD_ERROR} />
          )}
          {shareLabel}
        </button>
      </div>
      {uploadError && (
        <div className="error-display-upload-error" role="alert">
          <CircleAlert size={UI.ICON_SIZE.UPLOAD_ERROR} />
          <span>{uploadError}</span>
        </div>
      )}

      <LogShareModal {...modalProps} />
    </div>
  );
}

export function ErrorDisplay({ error, onRetry, onCancel, compact = false }: ErrorDisplayProps) {
  const { t } = useTranslation();
  // The screen must never be message-less, whatever upstream race produced an empty error.
  const message = error.trim() || t('error.flashFailed');

  if (compact) return <CompactErrorDisplay message={message} onRetry={onRetry} />;

  return (
    <div className="error-screen">
      <div className="error-screen__hero" aria-hidden="true">
        <span className="error-screen__ring" />
        <span className="error-screen__ring" />
        <CircleX className="error-screen__glyph" size={UI.ICON_SIZE.ERROR_HERO} strokeWidth={1.5} />
      </div>

      <div className="error-screen__main">
        <h2 className="error-screen__title">{t('flash.failed')}</h2>
        <p className="error-screen__hint">{message}</p>

        {(onRetry || onCancel) && (
          <div className="error-screen__buttons">
            {onCancel && (
              <button className="btn btn-secondary" onClick={onCancel}>
                {t('flash.cancel')}
              </button>
            )}
            {onRetry && (
              <button className="btn btn-primary" onClick={onRetry}>
                <RotateCcw size={UI.ICON_SIZE.BUTTON} />
                {t('flash.retry')}
              </button>
            )}
          </div>
        )}

        <LogUploadRemedy />
      </div>
    </div>
  );
}
