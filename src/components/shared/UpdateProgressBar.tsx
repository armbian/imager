// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import type { UpdateProgress } from '../../contexts/UpdateContext';
import { formatTransfer, percentOf } from '../../utils';

interface UpdateProgressBarProps {
  progress: UpdateProgress;
  variant: 'detail' | 'rail';
  label?: string;
}

export function UpdateProgressBar({ progress, variant, label }: UpdateProgressBarProps) {
  const { t } = useTranslation();
  const percent = percentOf(progress.downloaded, progress.total);
  const fillStyle = percent === null ? undefined : ({ '--update-progress': percent / 100 } as CSSProperties);
  const transfer = formatTransfer(progress.downloaded, progress.total, t);

  return (
    <div className={`update-progress update-progress--${variant}`}>
      {variant === 'detail' && (
        <div className="update-progress__text">
          <span className="update-progress__label">{label}</span>
          <span className="update-progress__figures">
            {percent === null ? transfer : `${transfer} · ${percent}%`}
          </span>
        </div>
      )}
      <div
        className="update-progress__track"
        role="progressbar"
        aria-label={label ?? t('update.railDownloading')}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent ?? undefined}
        aria-valuetext={transfer}
      >
        <div className={`update-progress__fill${percent === null ? ' is-indeterminate' : ''}`} style={fillStyle} />
      </div>
    </div>
  );
}
