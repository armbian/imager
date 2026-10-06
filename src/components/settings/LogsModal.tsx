// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { X, Copy, Check } from 'lucide-react';
import Ansi from 'ansi-to-html';
import { getLogs } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { stripAnsiCodes } from '../../utils';
import { ErrorDisplay } from '../shared/ErrorDisplay';
import { TIMING } from '../../config';

interface LogsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/** Sub-modal rendering session logs as an ANSI-colored terminal panel; loads lazily on open, returns null when closed,
 * renders via dangerouslySetInnerHTML+escapeXML, copy strips ANSI and reverts after TIMING.COPIED_NOTIFICATION; no exit animation, no Escape handler (intentional), overlay click closes. */
export function LogsModal({ isOpen, onClose }: LogsModalProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState<boolean>(false);
  const { data: logs, loading, error, reload } = useAsyncData(() => getLogs(), [isOpen], {
    immediate: isOpen,
    resetOnReload: true,
  });

  /** Copies the logs to the clipboard as plain text, flashing a confirmation. */
  const handleCopyLogs = async () => {
    try {
      if (!logs) return;
      const plainText = stripAnsiCodes(logs);
      await navigator.clipboard.writeText(plainText);
      setCopied(true);
      setTimeout(() => setCopied(false), TIMING.COPIED_NOTIFICATION);
    } catch (err) {
      console.error('Failed to copy logs:', err);
    }
  };

  // Converts ANSI-coded log text into HTML; escapeXML guards against injection.
  const ansiConverter = new Ansi({
    fg: '#FFF',
    bg: '#000',
    newline: true,
    escapeXML: true,
    stream: false,
  });

  if (!isOpen) return null;

  /** Renders body per load state: loading line, error line, or the terminal panel with its floating copy pill. */
  const renderBody = () => {
    if (error) {
      return <ErrorDisplay error={error} onRetry={reload} compact />;
    }

    if (loading || logs === null) {
      return <div className="logs-loading">{t('modal.loading')}</div>;
    }

    return (
      <pre
        className="logs-content"
        dangerouslySetInnerHTML={{
          __html: logs ? ansiConverter.toHtml(logs) : t('settings.noLogsAvailable'),
        }}
      />
    );
  };

  // Copy action is offered in the header once logs are loaded.
  const canCopy = !loading && !error && !!logs;

  // Portal to <body> so the fixed overlay escapes the animated settings shell
  // (whose transform creates a containing block that would otherwise trap it).
  return createPortal(
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal modal-content logs-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="logs-modal-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h2 id="logs-modal-title">{t('settings.viewLogs')}</h2>
          <button className="modal-close" onClick={onClose} aria-label="Close">
            <X size={20} />
          </button>
        </div>

        <div className="modal-body">{renderBody()}</div>

        {canCopy && (
          <div className="logs-modal__footer">
            <button
              type="button"
              className="logs-copy-button"
              onClick={handleCopyLogs}
              aria-label={copied ? t('settings.copied') : t('settings.copyLogs')}
            >
              {copied ? (
                <>
                  <Check size={16} />
                  <span>{t('settings.copied')}</span>
                </>
              ) : (
                <>
                  <Copy size={16} />
                  <span>{t('settings.copyLogs')}</span>
                </>
              )}
            </button>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
