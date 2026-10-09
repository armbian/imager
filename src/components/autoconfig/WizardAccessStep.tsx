// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { Github, Gitlab, KeyRound, Link2, Lock } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, SSH_KEY_FORGES, type SshKeySource } from '../../config/autoconfig';
import { keySourceError, type AccessMode } from '../../config/profileEditorModel';
import type { ProfileWizardState } from '../../hooks/useProfileWizard';
import { SegmentedControl } from '../settings/controls';
import { KeyCheckHint } from '../settings/editor/AccessGroup';
import { FieldHint } from '../settings/editor/EditorParts';
import { PasswordInput } from '../shared/PasswordInput';
import { WizardField, WizardMore, WizardTextField } from './WizardFields';

const FORGE_ICON = { github: Github, gitlab: Gitlab } as const;

export function WizardAccessStep({ wizard, uid }: { wizard: ProfileWizardState; uid: string }) {
  const { t } = useTranslation();
  const { model, more, attempted, keyCheck, patch, toggleMore } = wizard;
  const a = model.access;
  const forge = a.source === 'link' ? null : SSH_KEY_FORGES[a.source];
  const ForgeIcon = a.source === 'link' ? Link2 : FORGE_ICON[a.source];
  const value = forge ? a.keyUser : a.keyLink;
  const inputError = keySourceError(a);
  // An empty field only complains once Next was pressed; a malformed one complains as you type, as in the editor.
  const showError = !!inputError && (attempted || !!value.trim());

  return (
    <>
      <SegmentedControl<AccessMode>
        value={a.mode}
        ariaLabel={t('settings.autoconfig.wizard.access.title')}
        options={[
          { value: 'keys', label: t('settings.autoconfig.wizard.access.keys'), icon: <KeyRound aria-hidden="true" /> },
          { value: 'password', label: t('settings.autoconfig.wizard.access.passwordOnly'), icon: <Lock aria-hidden="true" /> },
        ]}
        onChange={(mode) => patch('access', { mode })}
      />
      {a.mode === 'keys' ? (
        <>
          <WizardField label={t('settings.autoconfig.editor.keysFrom')}>
            <SegmentedControl<SshKeySource>
              value={a.source}
              ariaLabel={t('settings.autoconfig.editor.keysFrom')}
              options={[
                { value: 'github', label: SSH_KEY_FORGES.github.label, icon: <Github aria-hidden="true" /> },
                { value: 'gitlab', label: SSH_KEY_FORGES.gitlab.label, icon: <Gitlab aria-hidden="true" /> },
                { value: 'link', label: t('settings.autoconfig.wizard.access.link'), icon: <Link2 aria-hidden="true" /> },
              ]}
              onChange={(source) => patch('access', { source })}
            />
          </WizardField>
          <WizardField
            label={forge ? t('settings.autoconfig.wizard.access.forgeUser', { forge: forge.label }) : t('settings.autoconfig.wizard.access.linkLabel')}
            htmlFor={`${uid}-keys`}
            hint={
              showError && inputError ? (
                <FieldHint tone="warn">{t(`settings.autoconfig.${inputError}`)}</FieldHint>
              ) : keyCheck.status !== 'idle' ? (
                <KeyCheckHint check={keyCheck} forge={forge?.label ?? null} />
              ) : forge ? undefined : (
                <FieldHint>{t('settings.autoconfig.wizard.access.linkHint')}</FieldHint>
              )
            }
          >
            <WizardTextField
              id={`${uid}-keys`}
              mono={!forge}
              value={value}
              icon={<ForgeIcon size={15} className="pe-input__icon" aria-hidden="true" />}
              placeholder={forge ? AUTOCONFIG_PLACEHOLDERS.USER_NAME : AUTOCONFIG_PLACEHOLDERS.SSH_KEYS_URL}
              invalid={showError}
              onChange={(next) => patch('access', forge ? { keyUser: next } : { keyLink: next })}
            />
          </WizardField>
        </>
      ) : (
        <p className="pw-line">{t('settings.autoconfig.wizard.access.passwordLine')}</p>
      )}
      <WizardMore label={t('settings.autoconfig.rootPassword')} open={!!more.access} onToggle={() => toggleMore('access')} />
      {more.access && (
        <WizardField
          label={t('settings.autoconfig.rootPassword')}
          htmlFor={`${uid}-root`}
          hint={<FieldHint>{t('settings.autoconfig.editor.rootHint')}</FieldHint>}
        >
          <PasswordInput
            id={`${uid}-root`}
            value={a.rootPassword}
            placeholder={t('settings.autoconfig.editor.rootPlaceholder')}
            onChange={(rootPassword) => patch('access', { rootPassword })}
          />
        </WizardField>
      )}
    </>
  );
}
