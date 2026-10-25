// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { Upload, ExternalLink, CircleAlert, CircleX, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLogUpload } from '../../hooks/useLogUpload';
import { UI } from '../../config';
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
  const { upload, uploading, url: pasteUrl, error: uploadError, open } = useLogUpload();

  return (
    <div className="error-display-compact">
      <div className="error-display-message">
        <CircleAlert size={18} />
        <span>{message}</span>
      </div>
      <div className="error-display-actions">
        {onRetry && (
          <button onClick={onRetry} className="btn btn-primary btn-sm">
            {t('errorDisplay.retry')}
          </button>
        )}
        {!pasteUrl ? (
          <button
            className="btn btn-secondary btn-sm"
            onClick={upload}
            disabled={uploading}
          >
            <Upload size={14} />
            {uploading ? t('errorDisplay.uploading') : t('errorDisplay.uploadLogs')}
          </button>
        ) : (
          <button className="btn btn-secondary btn-sm" onClick={open}>
            <ExternalLink size={14} />
            {t('errorDisplay.viewLogs')}
          </button>
        )}
      </div>
      {uploadError && (
        <div className="error-display-upload-error">
          <CircleAlert size={12} />
          <span>{uploadError}</span>
        </div>
      )}
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

        <LogUploadRemedy />

        {(onRetry || onCancel) && (
          <div className="error-screen__buttons">
            {onCancel && (
              <button className="btn btn-secondary" onClick={onCancel}>
                {t('flash.cancel')}
              </button>
            )}
            {onRetry && (
              <button className="btn btn-primary" onClick={onRetry}>
                <RotateCcw size={16} />
                {t('flash.retry')}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
