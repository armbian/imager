// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { ChevronDown, Monitor, Terminal, User } from 'lucide-react';
import { UI } from '../../config';
import { AUTOCONFIG_PLACEHOLDERS, USER_SHELLS } from '../../config/autoconfig';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import type { UserShell } from '../../types';
import { PasswordInput } from '../shared/PasswordInput';
import { SegmentedChoice } from '../shared/SegmentedChoice';
import { WizardField, WizardMore, WizardTextField } from './WizardFields';

const SEG_ICON = UI.ICON_SIZE.WIZARD_SEG;

export function WizardUserStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t } = useTranslation();
  const { draft, more, attempted, stepError, patch, toggleMore, shownError } = wizard;
  const user = draft.user;

  return (
    <>
      <SegmentedChoice
        value={user.mode}
        ariaLabel={t('settings.autoconfig.wizard.user.title')}
        options={[
          { value: 'now', label: t('settings.autoconfig.wizard.user.now'), icon: <User size={SEG_ICON} aria-hidden="true" /> },
          { value: 'later', label: t('settings.autoconfig.wizard.user.later'), icon: <Monitor size={SEG_ICON} aria-hidden="true" /> },
        ]}
        onChange={(mode) => patch('user', { mode })}
      />
      {user.mode === 'now' ? (
        <>
          <div className="pw-two">
            <WizardField label={t('settings.autoconfig.userName')} htmlFor={`${uid}-user`} error={shownError(stepError)}>
              <WizardTextField
                id={`${uid}-user`}
                mono
                value={user.name}
                icon={<User size={15} className="ac-input__icon" aria-hidden="true" />}
                placeholder={AUTOCONFIG_PLACEHOLDERS.USER_NAME}
                invalid={attempted && !!stepError}
                onChange={(name) => patch('user', { name })}
              />
            </WizardField>
            <WizardField label={t('settings.autoconfig.wizard.password')} htmlFor={`${uid}-upw`}>
              <PasswordInput id={`${uid}-upw`} value={user.password} onChange={(password) => patch('user', { password })} />
            </WizardField>
          </div>
          <WizardMore label={t('settings.autoconfig.wizard.user.more')} open={!!more.user} onToggle={() => toggleMore('user')} />
          {more.user && (
            <div className="pw-two">
              <WizardField label={t('settings.autoconfig.userRealName')} htmlFor={`${uid}-real`}>
                <WizardTextField
                  id={`${uid}-real`}
                  value={user.realName}
                  placeholder={AUTOCONFIG_PLACEHOLDERS.USER_REAL_NAME}
                  onChange={(realName) => patch('user', { realName })}
                />
              </WizardField>
              <WizardField label={t('settings.autoconfig.userShell')} htmlFor={`${uid}-shell`}>
                <div className="ac-input pw-input is-select">
                  <Terminal size={15} className="ac-input__icon" aria-hidden="true" />
                  <select
                    id={`${uid}-shell`}
                    value={user.shell}
                    onChange={(e) => patch('user', { shell: e.target.value as UserShell | '' })}
                  >
                    <option value="">{t('settings.autoconfig.wizard.user.shellDefault')}</option>
                    {USER_SHELLS.map((s) => (
                      <option key={s} value={s}>{s}</option>
                    ))}
                  </select>
                  <ChevronDown size={15} className="ac-input__chevron" aria-hidden="true" />
                </div>
              </WizardField>
            </div>
          )}
        </>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.user.laterLine')}</p>
      )}
    </>
  );
}
