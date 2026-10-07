// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { AutoconfigConfig, UserShell } from '../types';
import { isHttpUrl, trimmedOrUndefined as orUndefined } from '../utils';
import type { TFn } from '../utils/errorUtils';
import { SSH_KEY_FORGES, forgeKeysUrl, isValidKeyUser, trimStaticIp, type SshKeySource } from './autoconfig';

export const WIZARD_STEPS = ['name', 'network', 'user', 'locale', 'access', 'review'] as const;
export type WizardStep = (typeof WIZARD_STEPS)[number];

/** Steps the left column lists with a status, and the only ones that can be skipped */
export const WIZARD_ITEMS = ['network', 'user', 'locale', 'access'] as const satisfies readonly WizardStep[];
export type WizardItem = (typeof WIZARD_ITEMS)[number];

export const isWizardItem = (step: WizardStep): step is WizardItem => (WIZARD_ITEMS as readonly string[]).includes(step);

export const WIZARD_ERROR = {
  NAME: 'settings.autoconfig.nameRequired',
  SSID: 'settings.autoconfig.wizard.ssidRequired',
  ADDRESS: 'settings.autoconfig.wizard.addressRequired',
  USERNAME: 'settings.autoconfig.wizard.usernameRequired',
  KEY_LINK: 'settings.autoconfig.keyUrlInvalid',
  KEY_USER: 'settings.autoconfig.wizard.keyUserInvalid',
} as const;
export type WizardError = (typeof WIZARD_ERROR)[keyof typeof WIZARD_ERROR];

export const WIZARD_SUMMARY_SEPARATOR = { SIDE: ' · ', REVIEW: ', ' } as const;

export type NetMode = 'cable' | 'wifi' | 'both';
export type IpMode = 'auto' | 'fixed';
export type UserMode = 'now' | 'later';
export type LocaleMode = 'location' | 'choose';
export type AccessMode = 'keys' | 'password';

export interface WizardDraft {
  name: string;
  network: {
    mode: NetMode;
    ssid: string;
    key: string;
    country: string;
    ip: IpMode;
    address: string;
    mask: string;
    gateway: string;
    dns: string;
  };
  user: { mode: UserMode; name: string; password: string; realName: string; shell: UserShell | '' };
  locale: { mode: LocaleMode; timezone: string; locale: string };
  access: { mode: AccessMode; source: SshKeySource; keyUser: string; keyLink: string; rootPassword: string };
  skipped: Partial<Record<WizardItem, boolean>>;
}

export function emptyWizardDraft(defaults: { timezone: string; locale: string }): WizardDraft {
  return {
    name: '',
    network: { mode: 'wifi', ssid: '', key: '', country: '', ip: 'auto', address: '', mask: '', gateway: '', dns: '' },
    user: { mode: 'now', name: '', password: '', realName: '', shell: '' },
    locale: { mode: 'choose', timezone: defaults.timezone, locale: defaults.locale },
    access: { mode: 'keys', source: 'github', keyUser: '', keyLink: '', rootPassword: '' },
    skipped: {},
  };
}

export const hasWifi = (mode: NetMode) => mode !== 'cable';
export const hasCable = (mode: NetMode) => mode !== 'wifi';

function wizardKeyUrl(access: WizardDraft['access']): string {
  if (access.mode !== 'keys') return '';
  if (access.source === 'link') return access.keyLink.trim();
  return access.keyUser.trim() ? forgeKeysUrl(access.source, access.keyUser) : '';
}

/** What stops Next on a step, or null; the IP checks come from the static IP validator. */
export function wizardStepError(step: WizardStep, d: WizardDraft): WizardError | null {
  switch (step) {
    case 'name':
      return d.name.trim() ? null : WIZARD_ERROR.NAME;
    case 'network':
      if (hasWifi(d.network.mode) && !d.network.ssid.trim()) return WIZARD_ERROR.SSID;
      if (d.network.ip === 'fixed' && !d.network.address.trim()) return WIZARD_ERROR.ADDRESS;
      return null;
    case 'user':
      return d.user.mode === 'now' && !d.user.name.trim() ? WIZARD_ERROR.USERNAME : null;
    case 'access': {
      if (d.access.mode !== 'keys') return null;
      if (d.access.source === 'link') {
        return isHttpUrl(d.access.keyLink) ? null : WIZARD_ERROR.KEY_LINK;
      }
      return isValidKeyUser(d.access.source, d.access.keyUser) ? null : WIZARD_ERROR.KEY_USER;
    }
    default:
      return null;
  }
}

export function wizardStaticIp(n: WizardDraft['network']) {
  return { staticIp: n.address, staticMask: n.mask, staticGateway: n.gateway, staticDns: n.dns };
}

// Passwords are kept as typed: a leading or trailing space can be part of one.
const secretOrUndefined = (value: string) => (value ? value : undefined);

/** The profile config the wizard writes; skipped steps add nothing, so Armbian keeps its defaults. */
export function wizardConfig(d: WizardDraft): AutoconfigConfig {
  const config: AutoconfigConfig = {};
  if (!d.skipped.network) {
    const n = d.network;
    config.applyNetwork = true;
    config.ethernetEnabled = hasCable(n.mode);
    config.wifiEnabled = hasWifi(n.mode);
    if (hasWifi(n.mode)) {
      config.wifiSsid = orUndefined(n.ssid);
      config.wifiKey = secretOrUndefined(n.key);
      config.wifiCountryCode = orUndefined(n.country);
    }
    config.useStaticIp = n.ip === 'fixed';
    if (n.ip === 'fixed') Object.assign(config, trimStaticIp(wizardStaticIp(n)));
  }
  const userNow = !d.skipped.user && d.user.mode === 'now';
  if (userNow) {
    config.userName = orUndefined(d.user.name);
    config.userPassword = secretOrUndefined(d.user.password);
    config.userRealName = orUndefined(d.user.realName);
    config.userShell = d.user.shell || undefined;
  }
  if (!d.skipped.locale) {
    config.langBasedOnLocation = d.locale.mode === 'location';
    if (d.locale.mode === 'choose') {
      config.timezone = orUndefined(d.locale.timezone);
      config.locale = orUndefined(d.locale.locale);
    }
  }
  if (!d.skipped.access) {
    const url = wizardKeyUrl(d.access);
    // PRESET_USER_KEY needs the user this profile creates; without one the keys go to root.
    if (url) {
      if (config.userName) config.userKeyUrl = url;
      else config.rootKeyUrl = url;
    }
    config.rootPassword = secretOrUndefined(d.access.rootPassword);
  }
  return config;
}

function keysSummary(access: WizardDraft['access'], keyCount: number | null, t: TFn): string {
  if (access.source !== 'link') {
    const forge = SSH_KEY_FORGES[access.source].label;
    return keyCount === null
      ? t('settings.autoconfig.wizard.sum.keysFrom', { forge })
      : t('settings.autoconfig.wizard.sum.keysFromCount', { forge, n: keyCount });
  }
  return keyCount === null
    ? t('settings.autoconfig.wizard.sum.keysLink')
    : t('settings.autoconfig.wizard.sum.keysLinkCount', { n: keyCount });
}

/** What a step sets, as the left column and the review list it; empty when it keeps Armbian's defaults. */
export function wizardItemSummary(item: WizardItem, d: WizardDraft, keyCount: number | null, t: TFn): string[] {
  if (d.skipped[item]) return [];
  const parts = (values: (string | false)[]) => values.filter((v): v is string => !!v);
  switch (item) {
    case 'network': {
      const n = d.network;
      return parts([
        hasWifi(n.mode) && t('settings.autoconfig.wizard.sum.wifi', { ssid: n.ssid.trim() }),
        hasCable(n.mode) && t('settings.autoconfig.wizard.sum.cable'),
        n.ip === 'fixed' && n.address.trim(),
      ]);
    }
    case 'user':
      return d.user.mode === 'now' ? parts([d.user.name.trim(), d.user.shell]) : [];
    case 'locale':
      return d.locale.mode === 'location'
        ? [t('settings.autoconfig.wizard.sum.location')]
        : parts([d.locale.timezone, d.locale.locale.split('.')[0]]);
    case 'access':
      return parts([
        d.access.mode === 'keys' ? keysSummary(d.access, keyCount, t) : t('settings.autoconfig.wizard.sum.passwordOnly'),
        !!d.access.rootPassword && t('settings.autoconfig.wizard.sum.rootSet'),
      ]);
  }
}
