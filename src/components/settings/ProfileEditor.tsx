// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, Trash2 } from 'lucide-react';
import { keyLookupSource, localeForLanguage } from '../../config/autoconfig';
import {
  blockingReason,
  firstBlockingGroup,
  fromEditorModel,
  keySourceError,
  profileFacts,
  profileProgress,
  sameModel,
  toEditorModel,
  type EditorGroup,
  type EditorModel,
  type MoreKey,
} from '../../config/profileEditorModel';
import { useMotion } from '../../contexts/MotionContext';
import { deleteAutoconfigProfile, upsertAutoconfigProfile } from '../../hooks/useSettings';
import { useSshKeyCheck } from '../../hooks/useSshKeyCheck';
import { logWarn } from '../../hooks/useTauri';
import { useToasts } from '../../hooks/useToasts';
import type { AutoconfigProfile, LeaveGuard } from '../../types';
import { focusAfterInput } from '../../utils';
import { isSshKeysSourceError } from '../../utils/errorUtils';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { AccessGroup } from './editor/AccessGroup';
import { SettingsGroup, SettingsRow } from './controls';
import { ActionPill } from './editor/ActionPill';
import { AdvancedGroup } from './editor/AdvancedGroup';
import { EditorHeader } from './editor/EditorHeader';
import { NameScopeGroup } from './editor/NameScopeGroup';
import { NetworkGroup } from './editor/NetworkGroup';
import { RegionGroup } from './editor/RegionGroup';
import { UserGroup } from './editor/UserGroup';
import type { DefaultsPart, EditorPart } from './editor/types';

interface ProfileEditorProps {
  profile: AutoconfigProfile | null;
  onSaved: (profile: AutoconfigProfile) => void;
  onDeleted?: (id: string) => void;
  onCancel: () => void;
  registerLeaveGuard: (guard: LeaveGuard | null) => void;
}

const NO_MORE: Record<MoreKey, boolean> = { network: false, user: false, access: false };

function revealGroup(root: HTMLElement | null, group: EditorGroup, behavior: ScrollBehavior) {
  const section = root?.querySelector<HTMLElement>(`[data-group="${group}"]`);
  section?.scrollIntoView({ block: 'start', behavior });
  section?.querySelector<HTMLElement>('[aria-invalid="true"], input')?.focus({ preventScroll: true });
}

export function ProfileEditor({ profile, onSaved, onDeleted, onCancel, registerLeaveGuard }: ProfileEditorProps) {
  const { t, i18n } = useTranslation();
  const { showSuccess, showError } = useToasts();
  const uid = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const isNew = profile === null;
  const { reduced } = useMotion();
  const scrollBehavior: ScrollBehavior = reduced ? 'auto' : 'smooth';

  const [initial] = useState<EditorModel>(() =>
    toEditorModel(profile, {
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone ?? '',
      locale: localeForLanguage(i18n.language),
    })
  );
  const [saved, setSaved] = useState(initial);
  const [model, setModel] = useState(initial);
  const [more, setMore] = useState(NO_MORE);
  const [saving, setSaving] = useState(false);
  const [confirm, setConfirm] = useState<'delete' | 'discard' | null>(null);
  const [leaving, setLeaving] = useState<((ok: boolean) => void) | null>(null);

  const dirty = !sameModel(model, saved);
  const blocking = firstBlockingGroup(model);

  const access = model.access;
  const keyCheck = useSshKeyCheck(
    keyLookupSource(access.source),
    access.source === 'link' ? access.keyLink : access.keyUser,
    access.mode === 'keys' && keySourceError(access) === null
  );
  const keyProblem = keyCheck.status === 'failed' && isSshKeysSourceError(keyCheck.error);
  const keyCount = keyCheck.status === 'found' ? keyCheck.lookup.total : null;
  const progress = profileProgress(model, keyProblem);
  const facts = useMemo(() => profileFacts(model, keyCount, keyProblem, t), [model, keyCount, keyProblem, t]);

  const dirtyRef = useRef(dirty);
  useEffect(() => {
    dirtyRef.current = dirty;
  }, [dirty]);

  const guard = useCallback<LeaveGuard>(
    () => (dirtyRef.current ? new Promise<boolean>((resolve) => setLeaving(() => resolve)) : Promise.resolve(true)),
    []
  );
  useEffect(() => {
    registerLeaveGuard(guard);
    return () => registerLeaveGuard(null);
  }, [registerLeaveGuard, guard]);

  const set = (value: Partial<EditorModel>) => setModel((m) => ({ ...m, ...value }));
  const patch = <K extends EditorPart>(key: K, value: Partial<EditorModel[K]>) =>
    setModel((m) => {
      const next = { ...m[key], ...value };
      // Touching the network or region writes it, so an untouched profile keeps Armbian's defaults.
      if (key === 'network' || key === 'region') Object.assign(next, { apply: true });
      return { ...m, [key]: next };
    });
  // Back to the saved part when it was unapplied, so the round trip leaves nothing dirty
  const keepDefaults = (key: DefaultsPart, byPointer: boolean) => {
    setModel((m) => ({ ...m, [key]: saved[key].apply ? { ...m[key], apply: false } : saved[key] }));
    requestAnimationFrame(() => {
      const first = rootRef.current?.querySelector<HTMLElement>(`[data-group="${key}"] .set-seg__opt`);
      if (first) focusAfterInput(first, byPointer);
    });
  };
  const toggleMore = (key: MoreKey) => setMore((m) => ({ ...m, [key]: !m[key] }));

  const save = async () => {
    if (saving || (!isNew && !dirty)) return;
    if (blocking) {
      const { group, more: fold } = blocking;
      if (fold) setMore((m) => ({ ...m, [fold]: true }));
      requestAnimationFrame(() => revealGroup(rootRef.current, group, scrollBehavior));
      return;
    }
    setSaving(true);
    const next: AutoconfigProfile = {
      id: profile?.id ?? crypto.randomUUID(),
      updatedAt: Date.now(),
      ...fromEditorModel(model),
    };
    try {
      await upsertAutoconfigProfile(next);
      showSuccess(t(isNew ? 'settings.autoconfig.toastCreated' : 'settings.autoconfig.toastUpdated'));
      setSaved(model);
      dirtyRef.current = false;
      onSaved(next);
    } catch (error) {
      logWarn('autoconfig', `Failed to save profile: ${error}`);
      showError(t('settings.autoconfig.toastError'));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!profile) return;
    setConfirm(null);
    setSaving(true);
    try {
      await deleteAutoconfigProfile(profile.id);
      showSuccess(t('settings.autoconfig.toastDeleted'));
      dirtyRef.current = false;
      onDeleted?.(profile.id);
    } catch (error) {
      logWarn('autoconfig', `Failed to delete profile: ${error}`);
      showError(t('settings.autoconfig.toastError'));
      setSaving(false);
    }
  };

  const leave = () => (dirty ? setConfirm('discard') : onCancel());
  const resolveLeaving = (ok: boolean) => {
    leaving?.(ok);
    setLeaving(null);
  };

  const groupProps = { model, uid, patch, set, keepDefaults };

  return (
    <div ref={rootRef} className="pe">
      <button type="button" className="pe-back" onClick={leave}>
        <ChevronLeft size={16} aria-hidden="true" />
        {t('settings.nav.profiles')}
      </button>

      <EditorHeader model={model} isNew={isNew} progress={progress} facts={facts} />

      <div className="pe-form">
        <NameScopeGroup {...groupProps} status={progress.status.name} profileId={profile?.id} />
        <NetworkGroup {...groupProps} status={progress.status.network} more={more.network} toggleMore={toggleMore} />
        <UserGroup {...groupProps} status={progress.status.user} more={more.user} toggleMore={toggleMore} />
        <AccessGroup {...groupProps} status={progress.status.access} more={more.access} toggleMore={toggleMore} keyCheck={keyCheck} />
        <RegionGroup {...groupProps} status={progress.status.region} />
        <AdvancedGroup {...groupProps} status={progress.status.advanced} />
        {!isNew && (
          <SettingsGroup eyebrow={t('settings.writing.dangerZone')} danger>
            <SettingsRow
              danger
              label={t('settings.autoconfig.editor.deleteProfile')}
              description={t('settings.autoconfig.editor.deleteProfileDesc')}
              control={
                <button type="button" className="btn btn-danger btn-pill" disabled={saving} onClick={() => setConfirm('delete')}>
                  <Trash2 size={14} aria-hidden="true" />
                  {t('settings.autoconfig.delete')}
                </button>
              }
            />
          </SettingsGroup>
        )}
      </div>

      <ActionPill
        isNew={isNew}
        dirty={dirty}
        blockedReason={blockingReason(model, isNew)}
        saving={saving}
        onSave={save}
        onDiscard={() => setModel(saved)}
        onCancel={leave}
      />

      <ConfirmationDialog
        isOpen={confirm === 'delete'}
        title={t('settings.autoconfig.deleteConfirmTitle')}
        message={t('settings.autoconfig.deleteConfirmBody', { name: profile?.name ?? '' })}
        confirmText={t('settings.autoconfig.delete')}
        onCancel={() => setConfirm(null)}
        onConfirm={remove}
      />
      <ConfirmationDialog
        isOpen={confirm === 'discard' || leaving !== null}
        title={t('settings.profiles.discardTitle')}
        message={t('settings.profiles.discardBody')}
        confirmText={t('settings.autoconfig.editor.discard')}
        onCancel={() => (leaving ? resolveLeaving(false) : setConfirm(null))}
        onConfirm={() => {
          if (leaving) {
            resolveLeaving(true);
            return;
          }
          setConfirm(null);
          onCancel();
        }}
      />
    </div>
  );
}
