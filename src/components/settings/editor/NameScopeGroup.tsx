// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { BoardScopeBlock } from '../BoardScopeBlock';
import { SettingsRow } from '../controls';
import { EditorSection, FieldHint, TextField } from './EditorParts';
import { fieldId, type EditorGroupProps } from './types';

interface NameScopeGroupProps extends EditorGroupProps {
  profileId?: string;
}

export function NameScopeGroup({ model, status, uid, set, profileId }: NameScopeGroupProps) {
  const { t } = useTranslation();
  const nameId = fieldId(uid, 'name');
  const named = !!model.name.trim();

  return (
    <EditorSection
      id="name"
      eyebrow={t('settings.autoconfig.editor.groupName')}
      description={t('settings.autoconfig.editor.groupNameDesc')}
      attention={status === 'attn'}
    >
      <SettingsRow
        label={t('settings.autoconfig.nameLabel')}
        controlId={nameId}
        tall={!named}
        description={named ? undefined : <FieldHint tone="warn">{t('settings.autoconfig.nameRequired')}</FieldHint>}
        control={
          <TextField
            id={nameId}
            value={model.name}
            placeholder={t('settings.autoconfig.namePlaceholder')}
            onChange={(name) => set({ name })}
          />
        }
      />
      <BoardScopeBlock
        scope={model.scope}
        boards={model.boards}
        onScope={(scope) => set({ scope })}
        onBoards={(boards) => set({ boards })}
        onRemove={(slug) => set({ boards: model.boards.filter((b) => b.slug !== slug) })}
        profileId={profileId}
      />
    </EditorSection>
  );
}
