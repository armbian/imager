// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { IdCard, Monitor, Terminal, User } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, USER_SHELLS } from '../../config/autoconfig';
import { realNameIssue, realNamePlaceholder, userNameIssue, type UserMode } from '../../config/profileEditorModel';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import type { UserShell } from '../../types';
import { SegmentedControl, SelectMenu } from '../settings/controls';
import { FieldHint } from '../settings/editor/EditorParts';
import { UserNameHint } from '../settings/editor/EditorParts';
import { PasswordInput } from '../shared/PasswordInput';
import { WizardField, WizardMore, WizardTextField } from './WizardFields';

export function WizardUserStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t } = useTranslation();
  const { model, more, attempted, patch, toggleMore } = wizard;
  const user = model.user;
  const named = !!user.name.trim();
  const nameIssue = userNameIssue(user.name);
  const realNameError = realNameIssue(user.realName);
  const passwordMissing = attempted && !user.password;

  return (
    <>
      <SegmentedControl<UserMode>
        value={user.mode}
        ariaLabel={t('settings.autoconfig.wizard.user.title')}
        options={[
          { value: 'now', label: t('settings.autoconfig.wizard.user.now'), icon: <User aria-hidden="true" /> },
          { value: 'later', label: t('settings.autoconfig.wizard.user.later'), icon: <Monitor aria-hidden="true" /> },
        ]}
        onChange={(mode) => patch('user', { mode })}
      />
      {user.mode === 'now' ? (
        <>
          <div className="pw-two">
            <WizardField label={t('settings.autoconfig.userName')} htmlFor={`${uid}-user`} hint={named || attempted ? <UserNameHint name={user.name} /> : undefined}>
              <WizardTextField
                id={`${uid}-user`}
                mono
                value={user.name}
                icon={<User size={15} className="pe-input__icon" aria-hidden="true" />}
                placeholder={AUTOCONFIG_PLACEHOLDERS.USER_NAME}
                invalid={!!nameIssue && (attempted || named)}
                onChange={(name) => patch('user', { name })}
              />
            </WizardField>
            <WizardField
              label={t('settings.autoconfig.wizard.password')}
              htmlFor={`${uid}-upw`}
              hint={passwordMissing ? <FieldHint tone="warn">{t('settings.autoconfig.editor.passwordRequired')}</FieldHint> : undefined}
            >
              <PasswordInput
                id={`${uid}-upw`}
                value={user.password}
                invalid={passwordMissing}
                onChange={(password) => patch('user', { password })}
              />
            </WizardField>
          </div>
          <WizardMore label={t('settings.autoconfig.wizard.user.more')} open={!!more.user} onToggle={() => toggleMore('user')} />
          {more.user && (
            <div className="pw-two">
              <WizardField
                label={t('settings.autoconfig.userRealName')}
                htmlFor={`${uid}-real`}
                hint={realNameError ? <FieldHint tone="warn">{t(realNameError)}</FieldHint> : undefined}
              >
                <WizardTextField
                  id={`${uid}-real`}
                  value={user.realName}
                  icon={<IdCard size={15} className="pe-input__icon" aria-hidden="true" />}
                  placeholder={realNamePlaceholder(user.name)}
                  invalid={!!realNameError}
                  onChange={(realName) => patch('user', { realName })}
                />
              </WizardField>
              <WizardField label={t('settings.autoconfig.userShell')}>
                <SelectMenu<UserShell | ''>
                  value={user.shell}
                  options={[
                    { value: '', label: t('settings.autoconfig.wizard.user.shellDefault'), icon: <Terminal aria-hidden="true" /> },
                    ...USER_SHELLS.map((s) => ({ value: s, label: s, icon: <Terminal aria-hidden="true" /> })),
                  ]}
                  onChange={(shell) => patch('user', { shell })}
                  ariaLabel={t('settings.autoconfig.userShell')}
                />
              </WizardField>
            </div>
          )}
        </>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.user.laterStops')}</p>
      )}
    </>
  );
}
