// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { Github, Gitlab, KeyRound, Link2, Loader2, Lock } from 'lucide-react';
import { AUTOCONFIG_PLACEHOLDERS, SSH_KEY_FORGES, type SshKeySource } from '../../../config/autoconfig';
import { keySourceError, type AccessMode } from '../../../config/profileEditorModel';
import type { SshKeyCheck } from '../../../hooks/useSshKeyCheck';
import { isSshKeysSourceError, translateSshKeysError } from '../../../utils/errorUtils';
import { PasswordInput } from '../../shared/PasswordInput';
import { SegmentedControl, SettingsRow } from '../controls';
import { EditorSection, FieldHint, MoreRow, TextField } from './EditorParts';
import { fieldId, type AccessGroupProps } from './types';

const FORGE_ICON = { github: Github, gitlab: Gitlab } as const;

// Only a hint: the board fetches the keys itself at first boot.
export function KeyCheckHint({ check, forge }: { check: SshKeyCheck; forge: string | null }) {
  const { t } = useTranslation();
  switch (check.status) {
    case 'idle':
      return null;
    case 'checking':
      return (
        <span className="pe-hint is-plain" role="status">
          <Loader2 size={12} className="spinning" aria-hidden="true" />
          <span className="pe-hint__text">{t('settings.autoconfig.keys.checking')}</span>
        </span>
      );
    case 'found':
      return <FieldHint tone="ok">{t('settings.autoconfig.keys.found', { count: check.lookup.total })}</FieldHint>;
    case 'failed':
      return (
        <FieldHint tone={isSshKeysSourceError(check.error) ? 'warn' : 'plain'}>
          {translateSshKeysError(check.error, t, forge)}
        </FieldHint>
      );
  }
}

export function AccessGroup({ model, status, uid, patch, more, toggleMore, keyCheck }: AccessGroupProps) {
  const { t } = useTranslation();
  const a = model.access;
  const subId = fieldId(uid, 'access-more');
  const keyId = fieldId(uid, 'key');
  const forge = a.source === 'link' ? null : SSH_KEY_FORGES[a.source];
  const ForgeIcon = a.source === 'link' ? Link2 : FORGE_ICON[a.source];
  const inputError = keySourceError(a);

  return (
    <EditorSection
      id="access"
      eyebrow={t('settings.autoconfig.wizard.steps.access')}
      description={t('settings.autoconfig.wizard.access.sub')}
      attention={status === 'attn'}
    >
      <SettingsRow
        label={t('settings.autoconfig.editor.howYouGetIn')}
        tall={a.mode === 'password'}
        description={a.mode === 'password' ? <FieldHint>{t('settings.autoconfig.editor.passwordLoginHint')}</FieldHint> : undefined}
        control={
          <SegmentedControl<AccessMode>
            value={a.mode}
            ariaLabel={t('settings.autoconfig.editor.howYouGetIn')}
            options={[
              { value: 'keys', label: t('settings.autoconfig.wizard.access.keys'), icon: <KeyRound aria-hidden="true" /> },
              { value: 'password', label: t('settings.autoconfig.wizard.access.passwordOnly'), icon: <Lock aria-hidden="true" /> },
            ]}
            onChange={(mode) => patch('access', { mode })}
          />
        }
      />
      {a.mode === 'keys' && (
        <>
          <SettingsRow
            label={t('settings.autoconfig.editor.keysFrom')}
            control={
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
            }
          />
          <SettingsRow
            label={forge ? t('settings.autoconfig.wizard.access.forgeUser', { forge: forge.label }) : t('settings.autoconfig.wizard.access.linkLabel')}
            controlId={keyId}
            tall
            description={
              inputError ? (
                <FieldHint tone="warn">{t(`settings.autoconfig.${inputError}`)}</FieldHint>
              ) : (
                <KeyCheckHint check={keyCheck} forge={forge?.label ?? null} />
              )
            }
            control={
              <TextField
                id={keyId}
                mono={!forge}
                value={forge ? a.keyUser : a.keyLink}
                icon={<ForgeIcon size={16} className="pe-input__icon" aria-hidden="true" />}
                placeholder={forge ? AUTOCONFIG_PLACEHOLDERS.USER_NAME : AUTOCONFIG_PLACEHOLDERS.SSH_KEYS_URL}
                invalid={!!inputError && !!(forge ? a.keyUser : a.keyLink).trim()}
                onChange={(value) => patch('access', forge ? { keyUser: value } : { keyLink: value })}
              />
            }
          />
        </>
      )}
      <MoreRow
        label={t('settings.autoconfig.editor.moreOptions')}
        value={t(a.rootPassword ? 'settings.autoconfig.editor.rootSet' : 'settings.autoconfig.editor.rootNotSet')}
        open={more}
        controls={subId}
        onToggle={() => toggleMore('access')}
      />
      <div id={subId} className="pe-sub" hidden={!more}>
        <SettingsRow
          label={t('settings.autoconfig.rootPassword')}
          controlId={fieldId(uid, 'root')}
          tall
          description={<FieldHint>{t('settings.autoconfig.editor.rootHint')}</FieldHint>}
          control={
            <PasswordInput
              id={fieldId(uid, 'root')}
              value={a.rootPassword}
              placeholder={t('settings.autoconfig.editor.rootPlaceholder')}
              onChange={(rootPassword) => patch('access', { rootPassword })}
            />
          }
        />
      </div>
    </EditorSection>
  );
}
