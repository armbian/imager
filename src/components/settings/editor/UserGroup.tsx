// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { Monitor, User } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, USER_SHELLS } from '../../../config/autoconfig';
import { PROFILE_EDITOR, firstLoginUserName, isValidUserName, type UserMode } from '../../../config/profileEditorModel';
import type { UserShell } from '../../../types';
import { PasswordInput } from '../../shared/PasswordInput';
import { SegmentedControl, SelectMenu, SettingsRow } from '../controls';
import { EditorSection, FieldHint, MoreRow, TextField } from './EditorParts';
import { fieldId, type FoldingGroupProps } from './types';

export function UserGroup({ model, status, uid, patch, more, toggleMore }: FoldingGroupProps) {
  const { t } = useTranslation();
  const u = model.user;
  const subId = fieldId(uid, 'user-more');
  const createdName = firstLoginUserName(u.name);
  const nameHint = !u.name.trim() ? (
    <FieldHint tone="warn">{t('settings.autoconfig.wizard.usernameRequired')}</FieldHint>
  ) : isValidUserName(u.name) ? (
    <FieldHint tone="ok">
      {createdName === u.name.trim()
        ? t('settings.autoconfig.editor.userOk')
        : t('settings.autoconfig.editor.userCreatedAs', { name: createdName })}
    </FieldHint>
  ) : (
    <FieldHint tone="warn">{t('settings.autoconfig.editor.userInvalid')}</FieldHint>
  );
  const fullNameNeeded = !u.realName.trim() && model.region.mode === 'choose';
  const summary = [u.realName.trim() || t('settings.autoconfig.editor.fullNameNotSet'), u.shell]
    .filter(Boolean)
    .join(PROFILE_EDITOR.SEPARATOR);

  return (
    <EditorSection
      id="user"
      eyebrow={t('settings.autoconfig.wizard.steps.user')}
      description={t('settings.autoconfig.wizard.user.sub')}
      attention={status === 'attn'}
    >
      <SettingsRow
        label={t('settings.autoconfig.editor.whoLogsIn')}
        tall={u.mode === 'later'}
        description={u.mode === 'later' ? <FieldHint>{t('settings.autoconfig.editor.laterHint')}</FieldHint> : undefined}
        control={
          <SegmentedControl<UserMode>
            value={u.mode}
            ariaLabel={t('settings.autoconfig.editor.whoLogsIn')}
            options={[
              { value: 'now', label: t('settings.autoconfig.wizard.user.now'), icon: <User aria-hidden="true" /> },
              { value: 'later', label: t('settings.autoconfig.wizard.user.later'), icon: <Monitor aria-hidden="true" /> },
            ]}
            onChange={(mode) => patch('user', { mode })}
          />
        }
      />
      {u.mode === 'now' && (
        <>
          <SettingsRow
            label={t('settings.autoconfig.userName')}
            controlId={fieldId(uid, 'user')}
            tall
            description={nameHint}
            control={
              <TextField
                id={fieldId(uid, 'user')}
                value={u.name}
                placeholder={AUTOCONFIG_PLACEHOLDERS.USER_NAME}
                invalid={!!u.name.trim() && !isValidUserName(u.name)}
                onChange={(name) => patch('user', { name })}
              />
            }
          />
          <SettingsRow
            label={t('settings.autoconfig.wizard.password')}
            controlId={fieldId(uid, 'user-password')}
            tall={!u.password}
            description={u.password ? undefined : <FieldHint tone="warn">{t('settings.autoconfig.editor.passwordRequired')}</FieldHint>}
            control={
              <PasswordInput id={fieldId(uid, 'user-password')} value={u.password} onChange={(password) => patch('user', { password })} />
            }
          />
          <MoreRow
            label={t('settings.autoconfig.editor.moreOptions')}
            value={summary}
            open={more}
            controls={subId}
            onToggle={() => toggleMore('user')}
          />
          <div id={subId} className="pe-sub" hidden={!more}>
            <SettingsRow
              label={t('settings.autoconfig.userRealName')}
              controlId={fieldId(uid, 'full-name')}
              tall={!u.realName.trim()}
              description={
                u.realName.trim() ? undefined : (
                  <FieldHint tone={fullNameNeeded ? 'warn' : 'plain'}>{t('settings.autoconfig.editor.fullNameRequired')}</FieldHint>
                )
              }
              control={
                <TextField
                  id={fieldId(uid, 'full-name')}
                  value={u.realName}
                  placeholder={AUTOCONFIG_PLACEHOLDERS.USER_REAL_NAME}
                  onChange={(realName) => patch('user', { realName })}
                />
              }
            />
            <SettingsRow
              label={t('settings.autoconfig.userShell')}
              control={
                <SelectMenu<UserShell | ''>
                  value={u.shell}
                  options={[
                    { value: '', label: t('settings.autoconfig.wizard.user.shellDefault') },
                    ...USER_SHELLS.map((s) => ({ value: s, label: s })),
                  ]}
                  onChange={(shell) => patch('user', { shell })}
                  ariaLabel={t('settings.autoconfig.userShell')}
                />
              }
            />
          </div>
        </>
      )}
    </EditorSection>
  );
}
