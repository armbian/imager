// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { Upload, ExternalLink, CircleAlert, Loader2, ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLogUpload } from '../../hooks/useLogUpload';
import { UI } from '../../config';

interface LogPasteLinkProps {
  url: string;
  qr: string | null;
  onOpen: () => void;
}

function LogPasteLink({ url, qr, onOpen }: LogPasteLinkProps) {
  const { t } = useTranslation();

  return (
    <div className="error-screen__paste">
      {qr && <img src={qr} alt="" className="error-screen__qr" />}
      <div className="error-screen__paste-info">
        <span className="error-screen__paste-label">{t('errorDisplay.scanQrOrShare')}</span>
        <button type="button" className="paste-url" onClick={onOpen}>
          {url}
          <ExternalLink size={UI.ICON_SIZE.PASTE_LINK} />
        </button>
      </div>
    </div>
  );
}

/** Upload-logs row that turns into the QR and share link. */
export function LogUploadRemedy() {
  const { t } = useTranslation();
  const { upload, uploading, url, qr, error, open } = useLogUpload({ withQr: true });

  return (
    <>
      <div className="error-screen__card">
        {!url ? (
          <button
            type="button"
            className="error-screen__row"
            onClick={upload}
            disabled={uploading}
          >
            <span className="error-screen__chip">
              {uploading ? (
                <Loader2 size={UI.ICON_SIZE.REMEDY_ROW} className="spinning" />
              ) : (
                <Upload size={UI.ICON_SIZE.REMEDY_ROW} />
              )}
            </span>
            <span className="error-screen__label">
              {uploading
                ? t('errorDisplay.uploadingLogs')
                : t('errorDisplay.uploadLogsForSupport')}
            </span>
            {!uploading && <ArrowRight className="error-screen__arrow" size={UI.ICON_SIZE.BUTTON} />}
          </button>
        ) : (
          <LogPasteLink url={url} qr={qr} onOpen={open} />
        )}
      </div>

      {error && (
        <div className="error-screen__upload-error">
          <CircleAlert size={UI.ICON_SIZE.UPLOAD_ERROR} />
          <span>{error}</span>
        </div>
      )}
    </>
  );
}
