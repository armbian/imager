// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { keyLookupSource, localeForLanguage, presetIsEmpty } from '../config/autoconfig';
import {
  editorIpErrors, keySourceError, realNameIssue, wifiIssues, type EditorModel, type MoreKey,
} from '../config/profileEditorModel';
import {
  WIZARD_STEPS, emptyWizardModel, isWizardItem, wizardFacts, wizardModel, wizardProfile, wizardStepBlocked,
  wizardStepError, wizardVerdict, type WizardSkips,
} from '../config/profileWizard';
import type { AutoconfigProfile, ProfileBoard } from '../types';
import { isSshKeysSourceError } from '../utils/errorUtils';
import { upsertAutoconfigProfile } from './useSettings';
import { useSshKeyCheck } from './useSshKeyCheck';
import { logWarn } from './useTauri';
import { useToasts } from './useToasts';

type ModelPart = 'network' | 'user' | 'region' | 'access';

export function useProfileWizard(board: ProfileBoard | null, onSaved: (profile: AutoconfigProfile) => void) {
  const { t, i18n } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const [model, setModel] = useState<EditorModel>(() =>
    emptyWizardModel(
      { timezone: Intl.DateTimeFormat().resolvedOptions().timeZone ?? '', locale: localeForLanguage(i18n.language) },
      board
    )
  );
  const [skipped, setSkipped] = useState<WizardSkips>({});
  const [stepIndex, setStepIndex] = useState(0);
  const [furthest, setFurthest] = useState(0);
  const [more, setMore] = useState<Partial<Record<MoreKey, boolean>>>({});
  const [attempted, setAttempted] = useState(false);
  // Bumped on every refused Next or Create, so the step can focus the field to fix
  const [refusals, setRefusals] = useState(0);
  const [saving, setSaving] = useState(false);

  const step = WIZARD_STEPS[stepIndex];
  const stepError = wizardStepError(step, model);
  const blocked = wizardStepBlocked(step, model);
  // Every area left at Armbian's defaults: nothing to write, so it cannot be created
  const empty = useMemo(() => presetIsEmpty(wizardProfile(model, skipped).config), [model, skipped]);
  const ipErrors = editorIpErrors(model.network);

  const access = model.access;
  const keyCheck = useSshKeyCheck(
    keyLookupSource(access.source),
    access.source === 'link' ? access.keyLink : access.keyUser,
    access.mode === 'keys' && keySourceError(access) === null
  );
  const keyProblem = keyCheck.status === 'failed' && isSshKeysSourceError(keyCheck.error);
  const keyCount = keyCheck.status === 'found' ? keyCheck.lookup.total : null;
  const facts = useMemo(
    () => wizardFacts(model, skipped, keyCount, keyProblem, t),
    [model, skipped, keyCount, keyProblem, t]
  );
  const verdict = useMemo(() => wizardVerdict(model, skipped, t), [model, skipped, t]);

  const set = (value: Partial<EditorModel>) => setModel((m) => ({ ...m, ...value }));
  const patch = <K extends ModelPart>(key: K, value: Partial<EditorModel[K]>) =>
    setModel((m) => ({ ...m, [key]: { ...m[key], ...value } }));
  const toggleMore = (key: MoreKey) => setMore((m) => ({ ...m, [key]: !m[key] }));

  function go(index: number) {
    setAttempted(false);
    setStepIndex(index);
    setFurthest((f) => Math.max(f, index));
  }

  function next() {
    if (blocked) {
      setAttempted(true);
      setRefusals((r) => r + 1);
      // The Fixed IP fields sit under "more": open it so the reason Next refused is in view.
      const wifiProblem = Object.values(wifiIssues(model.network)).some(Boolean);
      if (step === 'network' && model.network.ip === 'fixed' && !wifiProblem) setMore((m) => ({ ...m, network: true }));
      // The full name sits under "more" too
      if (step === 'user' && stepError && stepError === realNameIssue(model.user.realName)) setMore((m) => ({ ...m, user: true }));
      return;
    }
    if (isWizardItem(step)) setSkipped((s) => ({ ...s, [step]: false }));
    go(stepIndex + 1);
  }

  function skip() {
    if (isWizardItem(step)) setSkipped((s) => ({ ...s, [step]: true }));
    go(stepIndex + 1);
  }

  async function create() {
    if (!model.name.trim() || saving || empty) return;
    // Edit from the review can leave an earlier step invalid; send the user back to it
    const saved = wizardModel(model, skipped);
    const invalidStep = WIZARD_STEPS.findIndex((s) => wizardStepBlocked(s, saved));
    if (invalidStep >= 0) {
      go(invalidStep);
      setAttempted(true);
      setRefusals((r) => r + 1);
      return;
    }
    setSaving(true);
    const profile: AutoconfigProfile = { id: crypto.randomUUID(), updatedAt: Date.now(), ...wizardProfile(model, skipped) };
    try {
      await upsertAutoconfigProfile(profile);
      showSuccess(t('settings.autoconfig.toastCreated'));
      onSaved(profile);
    } catch (error) {
      logWarn('autoconfig', `Failed to save the profile from the guided wizard: ${error}`);
      showError(t('settings.autoconfig.toastError'));
      setSaving(false);
    }
  }

  const shownError = attempted && stepError ? t(stepError) : null;

  return {
    model, skipped, step, stepIndex, furthest, more, attempted, refusals, saving, blocked, empty, stepError, shownError, ipErrors,
    keyCheck, facts, verdict, set, patch, toggleMore, go, next, skip, create,
  };
}

export type ProfileWizardState = ReturnType<typeof useProfileWizard>;
