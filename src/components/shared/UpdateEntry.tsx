// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { ChevronRight, CircleAlert, CircleCheck, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useUpdate } from '../../contexts/UpdateContext';
import { MarqueeText } from './MarqueeText';
import { UpdateProgressBar } from './UpdateProgressBar';
import { UI, UPDATER } from '../../config/constants';
import { formatTransfer, percentOf } from '../../utils';

// Sidebar entry under the MOTD tip: follows the update from offer to restart; the row opens Settings > About.
export function UpdateEntry({ onOpenDetails }: { onOpenDetails: () => void }) {
  const { t } = useTranslation();
  const { available, update, status, progress, failedAt, restartBlocked, download, restart, retry } = useUpdate();

  if (!available || !update) return null;

  const percent = percentOf(progress.downloaded, progress.total);
  const isReady = status === 'ready' || status === 'installing';
  const sub =
    status === 'downloading' && progress.downloaded > 0
      ? formatTransfer(progress.downloaded, progress.total, t)
      : `v${update.version}`;
  const Icon = isReady ? CircleCheck : status === 'error' ? CircleAlert : Download;

  let action = null;
  if (status === 'available') {
    action = (
      <button type="button" className="rail-update__cta" onClick={download}>
        {t('update.install')}
      </button>
    );
  } else if (isReady) {
    action = (
      <button
        type="button"
        className="rail-update__cta is-primary"
        onClick={restart}
        disabled={status === 'installing' || restartBlocked}
      >
        {t(status === 'installing' ? 'update.restarting' : 'update.restartNow')}
      </button>
    );
  } else if (status === 'error') {
    action = (
      <button
        type="button"
        className="rail-update__cta"
        onClick={retry}
        disabled={restartBlocked && failedAt !== 'download'}
      >
        {t('update.retry')}
      </button>
    );
  }

  return (
    <div className={`rail-update is-${status}`}>
      <button type="button" className="rail-update__open" onClick={onOpenDetails}>
        {/* Keyed so the swap animation replays per state without remounting the focused button */}
        <span className="rail-update__body" key={isReady ? 'ready' : status}>
          <span className="rail-update__icon" aria-hidden="true">
            <Icon size={UI.ICON_SIZE.UPDATE_ENTRY} />
          </span>
          <span className="rail-update__text">
            <MarqueeText text={t(UPDATER.TITLE_KEY[status])} className="rail-update__title" />
            <span className="rail-update__sub">{sub}</span>
          </span>
          {status === 'downloading' && (
            <span className="rail-update__trail" aria-hidden="true">
              {percent !== null && <span className="rail-update__percent">{percent}%</span>}
              <ChevronRight size={UI.ICON_SIZE.CHEVRON} className="rail-update__chevron" />
            </span>
          )}
        </span>
      </button>
      {action}
      {status === 'downloading' && <UpdateProgressBar progress={progress} variant="rail" />}
      <span className="visually-hidden" aria-live="polite">
        {t(UPDATER.ANNOUNCE_KEY[status])}
      </span>
    </div>
  );
}
