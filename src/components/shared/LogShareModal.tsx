// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Check, Copy, ExternalLink } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { LINKS, UI } from '../../config';
import { useCopyFeedback } from '../../hooks/useCopyFeedback';
import { logWarn, openExternal } from '../../hooks/useTauri';
import { splitUrlTail } from '../../utils';
import { Modal } from '../modals/Modal';

interface LogShareModalProps {
  isOpen: boolean;
  onClose: () => void;
  url: string | null;
  qr: string | null;
  onOpen: () => void;
}

export function LogShareModal({ isOpen, onClose, url, qr, onOpen }: LogShareModalProps) {
  const { t } = useTranslation();
  const { copied, copy } = useCopyFeedback(url);

  async function handleCopy() {
    try {
      await copy();
    } catch (err) {
      logWarn('log-share', `Failed to copy the paste link: ${err}`);
    }
  }

  const [urlHead, urlTail] = url ? splitUrlTail(url) : ['', ''];
  const copyLabel = copied ? t('errorDisplay.copied') : t('errorDisplay.copyLink');

  return (
    <Modal
      isOpen={isOpen && !!url}
      onClose={onClose}
      title={t('errorDisplay.logsUploaded')}
      className="log-share-modal"
      footer={(close) => (
        <button type="button" className="btn btn-primary btn-pill" onClick={close}>
          {t('common.done')}
        </button>
      )}
    >
      <div className="log-share__phone" aria-hidden="true">
        <span className="log-share__island" />
        <div className="log-share__screen">
          {qr && <img src={qr} alt="" className="log-share__qr" />}
          <span className="log-share__caption">{t('errorDisplay.logsCaption')}</span>
        </div>
      </div>

      <ol className="log-share__steps">
        <li>{t('errorDisplay.logsStepScan')}</li>
        <li>
          <button type="button" className="log-share__step-link" onClick={() => openExternal(LINKS.ISSUES)}>
            {t('errorDisplay.logsStepIssue')}
          </button>
        </li>
        <li>{t('errorDisplay.logsStepPaste')}</li>
      </ol>

      <div className="log-share__link">
        <span className="log-share__url" title={url ?? undefined}>
          {urlHead}
          <b>{urlTail}</b>
        </span>
        <button
          type="button"
          className="log-share__icon-btn"
          onClick={onOpen}
          aria-label={t('errorDisplay.openInBrowser')}
          title={t('errorDisplay.openInBrowser')}
        >
          <ExternalLink size={UI.ICON_SIZE.BUTTON} />
        </button>
        <button
          type="button"
          className={`log-share__icon-btn${copied ? ' is-copied' : ''}`}
          onClick={handleCopy}
          aria-label={copyLabel}
          title={copyLabel}
        >
          {copied ? <Check size={UI.ICON_SIZE.BUTTON} /> : <Copy size={UI.ICON_SIZE.BUTTON} />}
        </button>
        <span className="log-share__status" role="status">
          {copied ? t('errorDisplay.copied') : ''}
        </span>
      </div>
    </Modal>
  );
}
