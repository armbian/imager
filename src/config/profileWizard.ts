// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { AutoconfigProfile, ProfileBoard } from '../types';
import type { TFn } from '../utils/errorUtils';
import {
  editorIpErrors,
  fromEditorModel,
  hasWifi,
  isValidUserName,
  keySourceError,
  profileFacts,
  toEditorModel,
  type EditorModel,
  type ProfileFact,
} from './profileEditorModel';

export const WIZARD_STEPS = ['name', 'network', 'user', 'locale', 'access', 'review'] as const;
export type WizardStep = (typeof WIZARD_STEPS)[number];

/** Steps the left column lists with a status, and the only ones that can be skipped */
export const WIZARD_ITEMS = ['network', 'user', 'locale', 'access'] as const satisfies readonly WizardStep[];
export type WizardItem = (typeof WIZARD_ITEMS)[number];
export type WizardSkips = Partial<Record<WizardItem, boolean>>;

/** Columns of the Name step boards row: four boards per page */
export const WIZARD_BOARD_SLOTS = 4;

export const isWizardItem = (step: WizardStep): step is WizardItem => (WIZARD_ITEMS as readonly string[]).includes(step);

// Index of each item in the list profileFacts() returns.
const FACT_INDEX: Record<WizardItem, number> = { network: 0, user: 1, access: 2, locale: 3 };

export const wizardItemFact = (item: WizardItem, facts: readonly ProfileFact[]): ProfileFact => facts[FACT_INDEX[item]];

/** What an untouched or skipped item keeps, worded like the choice its step opens on */
export const WIZARD_DEFAULT_KEY: Record<WizardItem, string> = {
  network: 'settings.autoconfig.armbianDefaults',
  user: 'settings.autoconfig.wizard.user.later',
  locale: 'settings.autoconfig.armbianDefaults',
  access: 'settings.autoconfig.wizard.access.passwordOnly',
};

/** Each step opens on what Next would save untouched: Armbian's defaults (the editor's empty profile); a known board scopes it. */
export function emptyWizardModel(defaults: { timezone: string; locale: string }, board: ProfileBoard | null): EditorModel {
  return { ...toEditorModel(null, defaults), scope: board ? 'some' : 'all', boards: board ? [board] : [] };
}

/** i18n key of what stops Next on a step, or null; the static IP checks show on their fields */
export function wizardStepError(step: WizardStep, m: EditorModel): string | null {
  switch (step) {
    case 'name':
      if (!m.name.trim()) return 'settings.autoconfig.nameRequired';
      return m.scope === 'some' && !m.boards.length ? 'settings.autoconfig.editor.pickBoard' : null;
    case 'network':
      if (!m.network.apply) return null;
      if (hasWifi(m.network.mode) && !m.network.ssid.trim()) return 'settings.autoconfig.wizard.ssidRequired';
      return m.network.ip === 'fixed' && !m.network.address.trim() ? 'settings.autoconfig.wizard.addressRequired' : null;
    case 'user':
      if (m.user.mode !== 'now') return null;
      if (!m.user.name.trim()) return 'settings.autoconfig.wizard.usernameRequired';
      if (!isValidUserName(m.user.name)) return 'settings.autoconfig.editor.userInvalid';
      return m.user.password ? null : 'settings.autoconfig.editor.passwordRequired';
    case 'access': {
      const error = keySourceError(m.access);
      return error ? `settings.autoconfig.${error}` : null;
    }
    default:
      return null;
  }
}

export const wizardStepBlocked = (step: WizardStep, m: EditorModel): boolean =>
  !!wizardStepError(step, m) || (step === 'network' && m.network.apply && Object.keys(editorIpErrors(m.network)).length > 0);

/** A skipped step writes nothing, so Armbian keeps its own defaults there. */
export function wizardModel(m: EditorModel, skipped: WizardSkips): EditorModel {
  return {
    ...m,
    network: skipped.network ? { ...m.network, apply: false } : m.network,
    user: skipped.user ? { ...m.user, mode: 'later' } : m.user,
    region: skipped.locale ? { ...m.region, apply: false } : m.region,
    access: skipped.access ? { ...m.access, mode: 'password', rootPassword: '' } : m.access,
  };
}

export const wizardProfile = (m: EditorModel, skipped: WizardSkips): Pick<AutoconfigProfile, 'name' | 'config' | 'boards'> =>
  fromEditorModel(wizardModel(m, skipped));

/** profileFacts of what the wizard saves; a skipped area reads as the Armbian default it keeps */
export function wizardFacts(m: EditorModel, skipped: WizardSkips, keyCount: number | null, keyProblem: boolean, t: TFn): ProfileFact[] {
  const facts = profileFacts(wizardModel(m, skipped), keyCount, keyProblem, t);
  return facts.map((fact, index) => {
    const item = WIZARD_ITEMS.find((i) => FACT_INDEX[i] === index);
    return item && skipped[item] ? { ...fact, text: t(WIZARD_DEFAULT_KEY[item]), tone: 'plain' } : fact;
  });
}
