// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { CircleStop, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface FlashCancelledProps {
  partlyWritten: boolean;
  onRetry: () => void;
  onBack: () => void;
}

export function FlashCancelled({ partlyWritten, onRetry, onBack }: FlashCancelledProps) {
  const { t } = useTranslation();

  return (
    <div className={`error-screen is-cancelled${partlyWritten ? ' is-partial' : ''}`}>
      <div className="error-screen__hero" aria-hidden="true">
        <span className="error-screen__ring" />
        <span className="error-screen__ring" />
        <CircleStop className="error-screen__glyph" size={72} strokeWidth={1.5} />
      </div>

      <div className="error-screen__main">
        <div role="status">
          <h2 className="error-screen__title">{t('flash.cancelledTitle')}</h2>
          <p className="error-screen__hint">
            {partlyWritten ? t('flash.cancelledPartlyWritten') : t('flash.cancelledNothingWritten')}
          </p>
        </div>

        <div className="error-screen__buttons">
          <button className="btn btn-secondary" onClick={onBack}>
            {t('common.back')}
          </button>
          <button className="btn btn-primary" onClick={onRetry}>
            <RotateCcw size={16} />
            {t('flash.retry')}
          </button>
        </div>
      </div>
    </div>
  );
}
