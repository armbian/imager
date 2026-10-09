// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { Tag } from 'lucide-react';
import type { BoardScope } from '../../config/profileEditorModel';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import type { ProfileBoard } from '../../types';
import { WIZARD_BOARD_SLOTS } from '../../config/profileWizard';
import { BoardScopeBlock } from '../settings/BoardScopeBlock';
import { BoardsEmpty } from '../settings/BoardsEmpty';
import { FieldHint } from '../settings/editor/EditorParts';
import { WizardField, WizardTextField } from './WizardFields';

interface WizardNameStepProps {
  wizard: ProfileWizardState;
  uid: string;
  board: ProfileBoard | null;
}

export function WizardNameStep({ wizard, uid, board }: WizardNameStepProps) {
  const { t } = useTranslation();
  const { model, attempted, stepError, set, next } = wizard;
  const nameMissing = attempted && !model.name.trim();

  const setScope = (scope: BoardScope) =>
    set(scope === 'some' && !model.boards.length && board ? { scope, boards: [board] } : { scope });

  return (
    <>
      <WizardField
        label={t('settings.autoconfig.nameLabel')}
        htmlFor={`${uid}-name`}
        hint={
          nameMissing && stepError ? (
            <FieldHint tone="warn">{t(stepError)}</FieldHint>
          ) : (
            <FieldHint>{t('settings.autoconfig.wizard.nameHint')}</FieldHint>
          )
        }
      >
        <WizardTextField
          id={`${uid}-name`}
          value={model.name}
          autoFocus
          icon={<Tag size={15} className="pe-input__icon" aria-hidden="true" />}
          placeholder={t('settings.autoconfig.namePlaceholder')}
          invalid={nameMissing}
          onChange={(name) => set({ name })}
          onEnter={next}
        />
      </WizardField>

      <BoardScopeBlock
        scope={model.scope}
        boards={model.boards}
        slots={WIZARD_BOARD_SLOTS}
        onScope={setScope}
        onBoards={(boards) => set({ boards })}
        onRemove={(slug) => set({ boards: model.boards.filter((b) => b.slug !== slug) })}
        allContent={<BoardsEmpty title={t('settings.autoconfig.editor.scopeAll')} text={t('settings.autoconfig.wizard.allBoardsLine')} />}
      />
    </>
  );
}
