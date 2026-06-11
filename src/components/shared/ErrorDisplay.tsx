import { useState } from 'react';
import { Upload, ExternalLink, CircleAlert, CircleX, Loader2, ArrowRight, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { uploadLogs, openUrl } from '../../hooks/useTauri';
import QRCode from 'qrcode';
import { COLORS, QR_CODE } from '../../config';
import { getErrorMessage } from '../../utils';

interface ErrorDisplayProps {
  error: string;
  onRetry?: () => void;
  /** Dismiss/cancel handler; renders the secondary button in full-screen mode. */
  onCancel?: () => void;
  compact?: boolean;
}

export function ErrorDisplay({ error, onRetry, onCancel, compact = false }: ErrorDisplayProps) {
  const { t } = useTranslation();
  // The screen must never be message-less, whatever upstream race produced an empty error.
  const message = error.trim() || t('error.flashFailed');
  const [uploading, setUploading] = useState(false);
  const [pasteUrl, setPasteUrl] = useState<string | null>(null);
  const [qrCodeDataUrl, setQrCodeDataUrl] = useState<string | null>(null);
  const [uploadError, setUploadError] = useState<string | null>(null);

  async function handleUploadLogs() {
    setUploading(true);
    setUploadError(null);

    try {
      const result = await uploadLogs();
      setPasteUrl(result.url);

      if (!compact) {
        const qrDataUrl = await QRCode.toDataURL(result.url, {
          width: QR_CODE.WIDTH,
          margin: QR_CODE.MARGIN,
          color: {
            dark: COLORS.QR_DARK,
            light: COLORS.QR_LIGHT,
          },
        });
        setQrCodeDataUrl(qrDataUrl);
      }
    } catch (err) {
      setUploadError(getErrorMessage(err, t('error.uploadFailed')));
    } finally {
      setUploading(false);
    }
  }

  async function handleOpenUrl() {
    if (!pasteUrl) return;
    try {
      await openUrl(pasteUrl);
    } catch {
      window.open(pasteUrl, '_blank');
    }
  }

  if (compact) {
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
              onClick={handleUploadLogs}
              disabled={uploading}
            >
              <Upload size={14} />
              {uploading ? t('errorDisplay.uploading') : t('errorDisplay.uploadLogs')}
            </button>
          ) : (
            <button className="btn btn-secondary btn-sm" onClick={handleOpenUrl}>
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

  // Full-screen failure: two-column layout coherent with the offline screen —
  // an animated error hero on the left, the diagnosis + remedy on the right.
  return (
    <div className="error-screen">
      <div className="error-screen__hero" aria-hidden="true">
        <span className="error-screen__ring" />
        <span className="error-screen__ring" />
        <CircleX className="error-screen__glyph" size={72} strokeWidth={1.5} />
      </div>

      <div className="error-screen__main">
        <h2 className="error-screen__title">{t('flash.failed')}</h2>
        <p className="error-screen__hint">{message}</p>

        {/* Remedy card: one surface; upload-logs row turns into the QR + share link. */}
        <div className="error-screen__card">
          {!pasteUrl ? (
            <button
              type="button"
              className="error-screen__row"
              onClick={handleUploadLogs}
              disabled={uploading}
            >
              <span className="error-screen__chip">
                {uploading ? (
                  <Loader2 size={18} className="spinning" />
                ) : (
                  <Upload size={18} />
                )}
              </span>
              <span className="error-screen__label">
                {uploading
                  ? t('errorDisplay.uploadingLogs')
                  : t('errorDisplay.uploadLogsForSupport')}
              </span>
              {!uploading && <ArrowRight className="error-screen__arrow" size={16} />}
            </button>
          ) : (
            <div className="error-screen__paste">
              {qrCodeDataUrl && (
                <img src={qrCodeDataUrl} alt="QR Code" className="error-screen__qr" />
              )}
              <div className="error-screen__paste-info">
                <span className="error-screen__paste-label">
                  {t('errorDisplay.scanQrOrShare')}
                </span>
                <button className="paste-url" onClick={handleOpenUrl}>
                  {pasteUrl}
                  <ExternalLink size={12} />
                </button>
              </div>
            </div>
          )}
        </div>

        {uploadError && (
          <div className="error-screen__upload-error">
            <CircleAlert size={14} />
            <span>{uploadError}</span>
          </div>
        )}

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
