// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { AUTOCONFIG_PLACEHOLDERS } from '../../../config/autoconfig';
import { isHttpUrl } from '../../../utils';
import { SettingsRow } from '../controls';
import { EditorSection, FieldHint, TextField } from './EditorParts';
import { fieldId, type EditorGroupProps } from './types';

export function AdvancedGroup({ model, status, uid, set }: EditorGroupProps) {
  const { t } = useTranslation();
  const id = fieldId(uid, 'remote');
  const url = model.remote.trim();
  const bad = !!url && !isHttpUrl(url);

  return (
    <EditorSection
      id="advanced"
      eyebrow={t('settings.autoconfig.groupAdvanced')}
      description={t('settings.autoconfig.editor.groupAdvancedDesc')}
      attention={status === 'attn'}
    >
      <SettingsRow
        label={t('settings.autoconfig.remoteConfigUrl')}
        controlId={id}
        tall
        description={
          bad ? (
            <FieldHint tone="warn">{t('settings.autoconfig.urlInvalid')}</FieldHint>
          ) : (
            <FieldHint tone={url ? 'ok' : 'plain'}>{t('settings.autoconfig.editor.remoteHint')}</FieldHint>
          )
        }
        control={
          <TextField
            id={id}
            mono
            value={model.remote}
            placeholder={AUTOCONFIG_PLACEHOLDERS.REMOTE_CONFIG_URL}
            invalid={bad}
            onChange={(remote) => set({ remote })}
          />
        }
      />
    </EditorSection>
  );
}
