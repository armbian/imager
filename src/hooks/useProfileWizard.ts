// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { localeForLanguage, staticIpConfigErrors } from '../config/autoconfig';
import {
  WIZARD_ERROR, WIZARD_STEPS, emptyWizardDraft, isWizardItem, wizardConfig, wizardStaticIp, wizardStepError,
  type WizardDraft, type WizardStep,
} from '../config/profileWizard';
import type { AutoconfigProfile } from '../types';
import type { StaticIpErrors } from '../utils';
import { upsertAutoconfigProfile } from './useSettings';
import { logWarn } from './useTauri';
import { useToasts } from './useToasts';

type DraftGroup = 'network' | 'user' | 'locale' | 'access';

export function useProfileWizard(onSaved: (profile: AutoconfigProfile) => void) {
  const { t, i18n } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const [draft, setDraft] = useState<WizardDraft>(() =>
    emptyWizardDraft({
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone ?? '',
      locale: localeForLanguage(i18n.language),
    })
  );
  const [stepIndex, setStepIndex] = useState(0);
  const [more, setMore] = useState<Partial<Record<WizardStep, boolean>>>({});
  const [attempted, setAttempted] = useState(false);
  const [saving, setSaving] = useState(false);

  const step = WIZARD_STEPS[stepIndex];
  const fixedIp = draft.network.ip === 'fixed';
  const ipErrors: StaticIpErrors = fixedIp ? staticIpConfigErrors(wizardStaticIp(draft.network)) : {};
  const stepError = wizardStepError(step, draft);
  const blocked = !!stepError || (step === 'network' && Object.keys(ipErrors).length > 0);

  function patch<K extends DraftGroup>(key: K, value: Partial<WizardDraft[K]>) {
    setDraft((d) => ({ ...d, [key]: { ...d[key], ...value } }));
  }

  const setName = (name: string) => setDraft((d) => ({ ...d, name }));
  const toggleMore = (s: WizardStep) => setMore((m) => ({ ...m, [s]: !m[s] }));

  function go(index: number) {
    setAttempted(false);
    setStepIndex(index);
  }

  function next() {
    if (blocked) {
      setAttempted(true);
      // The Fixed IP fields sit under "more": open it so the reason Next refused is in view.
      if (step === 'network' && fixedIp) setMore((m) => ({ ...m, network: true }));
      return;
    }
    if (isWizardItem(step)) setDraft((d) => ({ ...d, skipped: { ...d.skipped, [step]: false } }));
    go(stepIndex + 1);
  }

  function skip() {
    if (isWizardItem(step)) setDraft((d) => ({ ...d, skipped: { ...d.skipped, [step]: true } }));
    go(stepIndex + 1);
  }

  async function create() {
    const name = draft.name.trim();
    if (!name || saving) return;
    setSaving(true);
    const profile: AutoconfigProfile = { id: crypto.randomUUID(), name, updatedAt: Date.now(), config: wizardConfig(draft) };
    try {
      await upsertAutoconfigProfile(profile);
      showSuccess(t('settings.autoconfig.toastCreated'));
      onSaved(profile);
    } catch {
      logWarn('autoconfig', 'Failed to save the profile from the guided wizard');
      showError(t('settings.autoconfig.toastError'));
      setSaving(false);
    }
  }

  const shownError = (key: string | null) => (attempted && key ? t(key) : null);
  const addressMissing = shownError(stepError === WIZARD_ERROR.ADDRESS ? stepError : null);
  const ipMessages = [addressMissing, ...Object.values(ipErrors).map((key) => t(`settings.autoconfig.${key}`))].filter(
    (message): message is string => !!message
  );

  return {
    draft, step, stepIndex, more, attempted, saving, blocked, stepError, ipErrors, ipMessages, addressMissing,
    patch, setName, toggleMore, go, next, skip, create, shownError,
  };
}

export type ProfileWizardState = ReturnType<typeof useProfileWizard>;
