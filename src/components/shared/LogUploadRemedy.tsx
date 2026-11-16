// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { Check, CircleAlert, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLogShare } from '../../hooks/useLogShare';
import { UI } from '../../config';
import { LogShareModal } from './LogShareModal';

/** Upload-logs footnote; once uploaded it reopens the share modal with the same link. */
export function LogUploadRemedy() {
  const { t } = useTranslation();
  const { share, uploading, uploaded, error, triggerRef, modalProps } = useLogShare();

  const label = uploading
    ? t('errorDisplay.uploadingLogs')
    : uploaded
      ? t('errorDisplay.showTheLink')
      : t('errorDisplay.uploadTheLogs');

  return (
    <>
      <div className="error-screen__footnote" aria-live="polite">
        {uploaded && (
          <Check className="error-screen__footnote-ok" size={UI.ICON_SIZE.UPLOAD_ERROR} aria-hidden="true" />
        )}
        <span>{uploaded ? t('errorDisplay.logsUploadedShort') : t('errorDisplay.needHelp')}</span>
        {/* Stays mounted across states so the share modal can hand focus back to it. */}
        <button
          ref={triggerRef}
          type="button"
          className="error-screen__footnote-action"
          onClick={share}
          disabled={uploading}
          aria-busy={uploading}
        >
          {uploading && <Loader2 size={UI.ICON_SIZE.UPLOAD_ERROR} className="spinning" aria-hidden="true" />}
          {label}
        </button>
      </div>

      {error && (
        <div className="error-screen__upload-error" role="alert">
          <CircleAlert size={UI.ICON_SIZE.UPLOAD_ERROR} />
          <span>{error}</span>
        </div>
      )}

      <LogShareModal {...modalProps} />
    </>
  );
}
