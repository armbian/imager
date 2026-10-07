// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Github, Gitlab, KeyRound, Link2, Lock } from 'lucide-react';
import { UI } from '../../config';
import { AUTOCONFIG_PLACEHOLDERS, SSH_KEY_FORGES, type SshKeySource } from '../../config/autoconfig';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { PasswordInput } from '../shared/PasswordInput';
import { SegmentedChoice } from '../shared/SegmentedChoice';
import { SshKeyStatus } from './SshKeyCheck';
import { WizardField, WizardMore, WizardTextField } from './WizardFields';

const SEG_ICON = UI.ICON_SIZE.WIZARD_SEG;
const FORGE_ICON = { github: Github, gitlab: Gitlab } as const;

export function WizardAccessStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t } = useTranslation();
  const { draft, more, attempted, stepError, keyCheck, patch, toggleMore, shownError } = wizard;
  const a = draft.access;
  const forge = a.source === 'link' ? null : SSH_KEY_FORGES[a.source];
  const ForgeIcon = a.source === 'link' ? null : FORGE_ICON[a.source];
  const keyStatus = keyCheck.status === 'idle' ? undefined : (
    <SshKeyStatus check={keyCheck} forge={forge?.label ?? null} showKeys />
  );
  const sources: { value: SshKeySource; label: string; icon: ReactNode }[] = [
    { value: 'github', label: SSH_KEY_FORGES.github.label, icon: <Github size={14} aria-hidden="true" /> },
    { value: 'gitlab', label: SSH_KEY_FORGES.gitlab.label, icon: <Gitlab size={14} aria-hidden="true" /> },
    { value: 'link', label: t('settings.autoconfig.wizard.access.link'), icon: <Link2 size={14} aria-hidden="true" /> },
  ];

  return (
    <>
      <SegmentedChoice
        value={a.mode}
        ariaLabel={t('settings.autoconfig.wizard.access.title')}
        options={[
          { value: 'keys', label: t('settings.autoconfig.wizard.access.keys'), icon: <KeyRound size={SEG_ICON} aria-hidden="true" /> },
          { value: 'password', label: t('settings.autoconfig.wizard.access.passwordOnly'), icon: <Lock size={SEG_ICON} aria-hidden="true" /> },
        ]}
        onChange={(mode) => patch('access', { mode })}
      />
      {a.mode === 'keys' ? (
        <>
          <div className="pw-chips" role="group" aria-label={t('settings.autoconfig.wizard.access.source')}>
            {sources.map((s) => (
              <button
                key={s.value}
                type="button"
                className={`pw-chip${a.source === s.value ? ' is-on' : ''}`}
                aria-pressed={a.source === s.value}
                onClick={() => patch('access', { source: s.value })}
              >
                {s.icon}
                {s.label}
              </button>
            ))}
          </div>
          {forge && ForgeIcon ? (
            <WizardField
              label={t('settings.autoconfig.wizard.access.forgeUser', { forge: forge.label })}
              htmlFor={`${uid}-keys`}
              error={shownError(stepError)}
              status={keyStatus}
            >
              <WizardTextField
                id={`${uid}-keys`}
                mono
                value={a.keyUser}
                prefix={forge.display}
                icon={<ForgeIcon size={15} className="ac-input__icon" aria-hidden="true" />}
                invalid={attempted && !!stepError}
                onChange={(keyUser) => patch('access', { keyUser })}
              />
            </WizardField>
          ) : (
            <WizardField
              label={t('settings.autoconfig.wizard.access.linkLabel')}
              htmlFor={`${uid}-link`}
              error={shownError(stepError)}
              status={keyStatus}
              hint={t('settings.autoconfig.wizard.access.linkHint')}
            >
              <WizardTextField
                id={`${uid}-link`}
                mono
                value={a.keyLink}
                icon={<Link2 size={15} className="ac-input__icon" aria-hidden="true" />}
                placeholder={AUTOCONFIG_PLACEHOLDERS.SSH_KEYS_URL}
                invalid={attempted && !!stepError}
                onChange={(keyLink) => patch('access', { keyLink })}
              />
            </WizardField>
          )}
        </>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.access.passwordLine')}</p>
      )}
      <WizardMore label={t('settings.autoconfig.rootPassword')} open={!!more.access} onToggle={() => toggleMore('access')} />
      {more.access && (
        <WizardField label={t('settings.autoconfig.rootPassword')} htmlFor={`${uid}-root`} hint={t('settings.autoconfig.wizard.access.rootHint')}>
          <PasswordInput id={`${uid}-root`} value={a.rootPassword} onChange={(rootPassword) => patch('access', { rootPassword })} />
        </WizardField>
      )}
    </>
  );
}
