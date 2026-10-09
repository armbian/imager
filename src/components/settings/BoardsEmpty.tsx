// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { ALL_BOARDS_ART } from '../../config/heroArt';

interface BoardsEmptyProps {
  title: string;
  text: string;
  /** The next step (Add board) and its hint; omitted, the panel only informs (the wizard's All boards) */
  action?: ReactNode;
  /** A board is required and none is picked: amber edge */
  warn?: boolean;
}

/** The boards stage: floating board art on a glow plate, and an eyebrow, title, line and action */
export function BoardsEmpty({ title, text, action, warn = false }: BoardsEmptyProps) {
  const { t } = useTranslation();
  return (
    <div className={`boards-empty${warn ? ' is-warn' : ''}`}>
      <span className="boards-empty__stage" aria-hidden="true">
        <img className="boards-empty__art" src={ALL_BOARDS_ART} alt="" />
      </span>
      <span className="boards-empty__text">
        <span className="boards-empty__eyebrow">{t('settings.autoconfig.editor.boardsEmptyEyebrow')}</span>
        <span className="boards-empty__title">{title}</span>
        <span className="boards-empty__line">{text}</span>
        {action && <span className="boards-empty__actions">{action}</span>}
      </span>
    </div>
  );
}
