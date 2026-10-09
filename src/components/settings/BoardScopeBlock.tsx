// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, useRef, useState, type MouseEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Cpu, Layers, Plus } from 'lucide-react';
import type { BoardScope } from '../../config/profileEditorModel';
import type { ProfileBoard } from '../../types';
import { BoardPickerSheet } from './BoardPickerSheet';
import { BoardsRow } from './BoardsRow';
import { SegmentedControl } from './controls';
import { FieldHint } from './editor/EditorParts';

interface BoardScopeBlockProps {
  scope: BoardScope;
  boards: ProfileBoard[];
  onScope: (scope: BoardScope) => void;
  onBoards: (boards: ProfileBoard[]) => void;
  onRemove: (slug: string) => void;
  /** The profile being edited, left out of the sheet's "Recent" brands */
  profileId?: string;
  /** Fixed grid columns (the wizard); omit to fit the width */
  slots?: number;
  /** Shown under the header with All boards (the wizard's panel); nothing when omitted */
  allContent?: ReactNode;
}

/** "Offered for" in one block, shared by the editor and the wizard: the state in the header, the boards under it */
export function BoardScopeBlock({ scope, boards, onScope, onBoards, onRemove, profileId, slots, allContent }: BoardScopeBlockProps) {
  const { t } = useTranslation();
  const labelId = useId();
  const some = scope === 'some';
  const [sheetOpen, setSheetOpen] = useState(false);
  // WebKit does not focus a clicked button, so the sheet is told which one opened it
  const openerRef = useRef<HTMLElement | null>(null);
  const addRef = useRef<HTMLButtonElement>(null);

  const openSheet = (event: MouseEvent<HTMLElement>) => {
    openerRef.current = event.currentTarget;
    setSheetOpen(true);
  };
  const closeSheet = () => {
    setSheetOpen(false);
    // The empty panel's button is gone once boards are picked: Add board takes the focus instead
    requestAnimationFrame(() => {
      if (!openerRef.current?.isConnected) addRef.current?.focus();
    });
  };

  return (
    <section className={`board-scope${some || allContent ? '' : ' board-scope--bare'}`} aria-labelledby={labelId}>
      <div className="board-scope__head">
        <span className="board-scope__text">
          <span id={labelId} className="board-scope__label">
            {t('settings.autoconfig.editor.offeredFor')}
          </span>
          {!some ? (
            <FieldHint>{t('settings.autoconfig.editor.scopeAll')}</FieldHint>
          ) : boards.length ? (
            <span className="board-scope__state">
              <FieldHint>{t('settings.autoconfig.editor.selectedCount', { count: boards.length })}</FieldHint>
              <button ref={addRef} type="button" className="board-scope__add" onClick={openSheet}>
                <Plus size={12} aria-hidden="true" />
                {t('settings.autoconfig.editor.addBoard')}
              </button>
            </span>
          ) : (
            <FieldHint tone="warn">{t('settings.autoconfig.editor.pickBoard')}</FieldHint>
          )}
        </span>
        <SegmentedControl<BoardScope>
          value={scope}
          ariaLabel={t('settings.autoconfig.editor.offeredFor')}
          options={[
            { value: 'all', label: t('settings.profiles.allBoards'), icon: <Layers aria-hidden="true" /> },
            { value: 'some', label: t('settings.autoconfig.editor.specificBoards'), icon: <Cpu aria-hidden="true" /> },
          ]}
          onChange={onScope}
        />
      </div>
      {some ? (
        <BoardsRow boards={boards} onEdit={openSheet} onRemove={onRemove} slots={slots} required />
      ) : (
        allContent
      )}
      {sheetOpen && (
        <BoardPickerSheet
          selected={boards}
          profileId={profileId}
          returnFocusRef={openerRef}
          onDone={(picked) => {
            onBoards(picked);
            closeSheet();
          }}
          onCancel={closeSheet}
        />
      )}
    </section>
  );
}
