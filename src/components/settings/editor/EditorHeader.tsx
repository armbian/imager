// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import {
  EDITOR_GROUPS,
  PROFILE_EDITOR,
  type EditorModel,
  type ProfileFact,
  type ProfileProgress,
} from '../../../config/profileEditorModel';
import { ProfileTile } from '../ProfileTile';
import { FactGlyph } from './FactGlyph';

interface EditorHeaderProps {
  model: EditorModel;
  isNew: boolean;
  progress: ProfileProgress;
  facts: ProfileFact[];
}

export function EditorHeader({ model, isNew, progress, facts }: EditorHeaderProps) {
  const { t } = useTranslation();
  const boards = model.scope === 'some' ? model.boards : [];
  const title = model.name.trim() || t(isNew ? 'settings.autoconfig.newProfile' : 'settings.autoconfig.editor.untitled');
  const scope =
    model.scope === 'all'
      ? t('settings.profiles.allBoards')
      : boards.length === 0
        ? t('settings.autoconfig.editor.noBoardPicked')
        : boards.length === 1
          ? boards[0].name
          : t('settings.profiles.boardCount', { count: boards.length });

  return (
    <div className="pe-head">
      <div className="pe-head__id">
        <span className="pe-head__tile">
          <ProfileTile boards={boards} size={PROFILE_EDITOR.TILE_SIZE} />
        </span>
        <div className="pe-head__text">
          <h2 className="pe-head__title">{title}</h2>
          <span className="pe-head__scope">{scope}</span>
          <span
            className="pe-head__progress"
            role="img"
            aria-label={t('settings.autoconfig.editor.sectionsSetLabel', { done: progress.done, total: progress.total })}
          >
            {EDITOR_GROUPS.map((g) => (
              <i key={g} className={`pe-head__seg is-${progress.status[g]}`} />
            ))}
            <em>{t('settings.autoconfig.editor.sectionsSet', { done: progress.done, total: progress.total })}</em>
          </span>
        </div>
      </div>
      <div className="pe-head__boot">
        <h3 className="pe-head__kicker">{t('settings.autoconfig.editor.atFirstBoot')}</h3>
        <ul className="pe-head__facts">
          {facts.map((f) => (
            <li key={f.icon} className={`pe-fact is-${f.tone}`}>
              <FactGlyph icon={f.icon} size={14} />
              <span>{f.text}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
