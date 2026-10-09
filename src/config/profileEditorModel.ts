// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { AutoconfigConfig, AutoconfigProfile, ProfileBoard, UserShell } from '../types';
import { isHttpUrl, trimmedOrUndefined } from '../utils';
import type { TFn } from '../utils/errorUtils';
import { UI } from './constants';
import {
  SSH_KEY_FORGES,
  forgeKeysUrl,
  isValidKeyUser,
  parseForgeKeysUrl,
  presetIsEmpty,
  profileBoards,
  staticIpConfigErrors,
  trimStaticIp,
  type SshKeySource,
} from './autoconfig';

export type BoardScope = 'all' | 'some';
export type NetMode = 'cable' | 'wifi' | 'both';
export type IpMode = 'auto' | 'fixed';
export type UserMode = 'now' | 'later';
export type RegionMode = 'location' | 'choose';
export type AccessMode = 'keys' | 'password';

export const EDITOR_GROUPS = ['name', 'network', 'user', 'access', 'region', 'advanced'] as const;
export type EditorGroup = (typeof EDITOR_GROUPS)[number];
export type GroupStatus = 'set' | 'attn' | 'none';
export type MoreKey = 'network' | 'user' | 'access';

export const PROFILE_EDITOR = {
  /** Board tile in the editor header, px */
  TILE_SIZE: 96,
  SEPARATOR: UI.SUMMARY_SEPARATOR,
} as const;

// Debian adduser's default NAME_REGEX, without the trailing "$" of machine accounts.
const USER_NAME_PATTERN = /^[a-z][-a-z0-9_]*$/;

/** The account armbian-firstlogin creates: lowercased, anything but [a-z0-9_-] dropped. */
export function firstLoginUserName(name: string): string {
  return name.trim().toLowerCase().replace(/[^a-z0-9_-]/g, '');
}

export function isValidUserName(name: string): boolean {
  return USER_NAME_PATTERN.test(firstLoginUserName(name));
}

export interface EditorModel {
  name: string;
  scope: BoardScope;
  boards: ProfileBoard[];
  /** `apply` false writes nothing, so Armbian keeps its own defaults; the values stay for when it is applied again */
  network: {
    apply: boolean;
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
  /** `extraRootKeyUrl` keeps a root key that differs from the user's, which the editor does not show */
  access: { mode: AccessMode; source: SshKeySource; keyUser: string; keyLink: string; rootPassword: string; extraRootKeyUrl: string };
  region: { apply: boolean; mode: RegionMode; locale: string; timezone: string };
  remote: string;
}

export const hasWifi = (mode: NetMode) => mode !== 'cable';
const hasCable = (mode: NetMode) => mode !== 'wifi';

function splitKeyUrl(url: string): Pick<EditorModel['access'], 'source' | 'keyUser' | 'keyLink'> {
  const parsed = parseForgeKeysUrl(url);
  if (parsed) return { source: parsed.forge, keyUser: parsed.user, keyLink: '' };
  return { source: url ? 'link' : 'github', keyUser: '', keyLink: url };
}

export function toEditorModel(profile: AutoconfigProfile | null, defaults: { timezone: string; locale: string }): EditorModel {
  const c: AutoconfigConfig = profile?.config ?? {};
  const boards = profile ? profileBoards(profile) : [];
  const net = !!c.applyNetwork;
  const keyUrl = c.userKeyUrl || c.rootKeyUrl || '';
  const regionSet = c.langBasedOnLocation !== undefined || !!c.locale || !!c.timezone;
  const chooses = c.langBasedOnLocation === false || (!c.langBasedOnLocation && (!!c.locale || !!c.timezone));
  return {
    name: profile?.name ?? '',
    scope: boards.length ? 'some' : 'all',
    boards,
    network: {
      apply: net,
      mode: net && c.wifiEnabled ? (c.ethernetEnabled ? 'both' : 'wifi') : 'cable',
      ssid: c.wifiSsid ?? '',
      key: c.wifiKey ?? '',
      country: c.wifiCountryCode ?? '',
      ip: net && c.useStaticIp ? 'fixed' : 'auto',
      address: c.staticIp ?? '',
      mask: c.staticMask ?? '',
      gateway: c.staticGateway ?? '',
      dns: c.staticDns ?? '',
    },
    user: {
      mode: c.userName?.trim() ? 'now' : 'later',
      name: c.userName ?? '',
      password: c.userPassword ?? '',
      realName: c.userRealName ?? '',
      shell: c.userShell ?? '',
    },
    access: {
      mode: keyUrl ? 'keys' : 'password',
      ...splitKeyUrl(keyUrl),
      rootPassword: c.rootPassword ?? '',
      extraRootKeyUrl: c.userKeyUrl && c.rootKeyUrl && c.rootKeyUrl !== c.userKeyUrl ? c.rootKeyUrl : '',
    },
    region: {
      apply: regionSet,
      mode: chooses ? 'choose' : 'location',
      locale: c.locale ?? defaults.locale,
      timezone: c.timezone ?? defaults.timezone,
    },
    remote: c.remoteConfigUrl ?? '',
  };
}

// Passwords, the Wi-Fi key and the SSID are kept as typed: a leading or trailing space can be part of one.
const asTyped = (value: string) => (value ? value : undefined);

function editorKeyUrl(access: EditorModel['access']): string {
  if (access.mode !== 'keys') return '';
  if (access.source === 'link') return access.keyLink.trim();
  return access.keyUser.trim() ? forgeKeysUrl(access.source, access.keyUser) : '';
}

function editorConfig(m: EditorModel): AutoconfigConfig {
  const config: AutoconfigConfig = {};
  const n = m.network;
  if (n.apply) {
    config.applyNetwork = true;
    config.ethernetEnabled = hasCable(n.mode);
    config.wifiEnabled = hasWifi(n.mode);
    if (hasWifi(n.mode)) {
      config.wifiSsid = asTyped(n.ssid);
      config.wifiKey = asTyped(n.key);
      config.wifiCountryCode = trimmedOrUndefined(n.country);
    }
    config.useStaticIp = n.ip === 'fixed';
    if (n.ip === 'fixed') {
      Object.assign(config, trimStaticIp({ staticIp: n.address, staticMask: n.mask, staticGateway: n.gateway, staticDns: n.dns }));
    }
  }
  if (m.user.mode === 'now') {
    config.userName = trimmedOrUndefined(m.user.name);
    config.userPassword = asTyped(m.user.password);
    config.userRealName = trimmedOrUndefined(m.user.realName);
    config.userShell = m.user.shell || undefined;
  }
  if (m.region.apply) {
    config.langBasedOnLocation = m.region.mode === 'location';
    if (m.region.mode === 'choose') {
      config.locale = trimmedOrUndefined(m.region.locale);
      config.timezone = trimmedOrUndefined(m.region.timezone);
    }
  }
  const keyUrl = editorKeyUrl(m.access);
  // PRESET_USER_KEY needs the user this profile creates; without one the keys go to root.
  if (keyUrl && config.userName) {
    config.userKeyUrl = keyUrl;
    config.rootKeyUrl = m.access.extraRootKeyUrl || undefined;
  } else if (keyUrl) {
    config.rootKeyUrl = keyUrl;
  }
  config.rootPassword = asTyped(m.access.rootPassword);
  config.remoteConfigUrl = trimmedOrUndefined(m.remote);
  return Object.fromEntries(Object.entries(config).filter(([, v]) => v !== undefined)) as AutoconfigConfig;
}

export function fromEditorModel(m: EditorModel): Pick<AutoconfigProfile, 'name' | 'config' | 'boards'> {
  return {
    name: m.name.trim(),
    config: editorConfig(m),
    boards: m.scope === 'some' && m.boards.length ? m.boards : undefined,
  };
}

/** i18n key (under settings.autoconfig) of what is wrong with the key source input, or null */
export function keySourceError(access: EditorModel['access']): string | null {
  if (access.mode !== 'keys') return null;
  if (access.source === 'link') {
    if (!access.keyLink.trim()) return 'editor.keyLinkRequired';
    return isHttpUrl(access.keyLink) ? null : 'keys.invalidLink';
  }
  if (!access.keyUser.trim()) return 'editor.keyUserRequired';
  return isValidKeyUser(access.source, access.keyUser) ? null : 'wizard.keyUserInvalid';
}

export function editorIpErrors(n: EditorModel['network']) {
  return n.ip === 'fixed' ? staticIpConfigErrors({ staticIp: n.address, staticMask: n.mask, staticGateway: n.gateway, staticDns: n.dns }) : {};
}

function networkBlocked(n: EditorModel['network']): boolean {
  if (!n.apply) return false;
  if (hasWifi(n.mode) && !n.ssid.trim()) return true;
  if (n.ip !== 'fixed') return false;
  return !n.address.trim() || Object.keys(editorIpErrors(n)).length > 0;
}

export interface ProfileProgress {
  status: Record<EditorGroup, GroupStatus>;
  done: number;
  total: number;
}

/** Per group: set, needs attention, or left at Armbian's defaults; `keyProblem` is a key source that serves no keys */
export function profileProgress(m: EditorModel, keyProblem = false): ProfileProgress {
  const n = m.network;
  const status: Record<EditorGroup, GroupStatus> = {
    name: m.name.trim() && (m.scope === 'all' || m.boards.length) ? 'set' : 'attn',
    network: !n.apply ? 'none' : networkBlocked(n) ? 'attn' : 'set',
    user: m.user.mode === 'later' ? 'none' : isValidUserName(m.user.name) && m.user.password ? 'set' : 'attn',
    access: m.access.mode === 'password' ? 'none' : keySourceError(m.access) || keyProblem ? 'attn' : 'set',
    region: !m.region.apply ? 'none' : m.region.mode === 'location' || (m.region.locale && m.region.timezone) ? 'set' : 'attn',
    advanced: !m.remote.trim() ? 'none' : isHttpUrl(m.remote) ? 'set' : 'attn',
  };
  const done = EDITOR_GROUPS.filter((g) => status[g] === 'set').length;
  return { status, done, total: EDITOR_GROUPS.length };
}

/** The first group that stops a save, with the folded "More options" that holds the problem; null when it can save */
export function firstBlockingGroup(m: EditorModel): { group: EditorGroup; more?: MoreKey; empty?: boolean } | null {
  if (!m.name.trim() || (m.scope === 'some' && !m.boards.length)) return { group: 'name' };
  if (networkBlocked(m.network)) {
    const inMore = !(hasWifi(m.network.mode) && !m.network.ssid.trim());
    return { group: 'network', more: inMore ? 'network' : undefined };
  }
  if (m.user.mode === 'now' && !(isValidUserName(m.user.name) && m.user.password)) return { group: 'user' };
  if (keySourceError(m.access)) return { group: 'access' };
  if (m.remote.trim() && !isHttpUrl(m.remote)) return { group: 'advanced' };
  // A profile that leaves every area at Armbian's defaults would write an empty preset
  if (presetIsEmpty(editorConfig(m))) return { group: 'network', empty: true };
  return null;
}

/** i18n key of why the profile cannot be saved (or created) yet, from the first blocking field; null when it can */
export function blockingReason(m: EditorModel, isNew: boolean): string | null {
  const blocking = firstBlockingGroup(m);
  if (!blocking) return null;
  const key = !m.name.trim() ? 'Name' : blocking.group === 'name' ? 'Board' : blocking.empty ? 'Something' : 'Fix';
  if (isNew && key === 'Name') return 'settings.autoconfig.editor.addNameToCreate';
  return `settings.autoconfig.editor.${isNew ? 'create' : 'save'}Needs${key}`;
}

export function sameModel(a: EditorModel, b: EditorModel): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** A country named in the UI language; the English name where the WebView has no Intl.DisplayNames */
export function countryName(code: string, englishName: string, uiLanguage: string): string {
  try {
    return new Intl.DisplayNames([uiLanguage], { type: 'region' }).of(code) ?? englishName;
  } catch {
    return englishName;
  }
}

/** The country part of a glibc locale ("it_IT.UTF-8" gives IT), or '' for C.UTF-8 */
export function localeCountry(locale: string): string {
  return /^[a-z]+_([A-Z]{2})\b/.exec(locale)?.[1] ?? '';
}

export type FactIcon = 'cable' | 'wifi' | 'user' | 'keys' | 'lock' | 'globe';
/** ok: applied as shown; plain: Armbian asks or decides; off: set but not applied */
type FactTone = 'ok' | 'plain' | 'off';
export interface ProfileFact {
  icon: FactIcon;
  text: string;
  tone: FactTone;
}

/** What the board does at first boot with this profile, one line per area; `keyCount` is null until a check finds keys */
export function profileFacts(m: EditorModel, keyCount: number | null, keyProblem: boolean, t: TFn): ProfileFact[] {
  const sep = PROFILE_EDITOR.SEPARATOR;
  const n = m.network;
  const address = n.ip === 'fixed' ? n.address.trim() : '';
  const join = (parts: (string | false)[]) => parts.filter(Boolean).join(sep);
  const netText =
    n.mode === 'cable'
      ? join([t('settings.autoconfig.wizard.network.cable'), address || t('settings.autoconfig.editor.factDhcp')])
      : n.mode === 'wifi'
        ? join([t('settings.autoconfig.wizard.network.wifi'), n.ssid.trim(), address])
        : address
          ? t('settings.autoconfig.editor.factBothFixed', { address })
          : t('settings.autoconfig.editor.factCableWifi');

  const userNow = m.user.mode === 'now' && !!m.user.name.trim();
  const a = m.access;
  const forge = a.source === 'link' ? null : SSH_KEY_FORGES[a.source].label;
  let access: ProfileFact;
  if (a.mode === 'password') access = { icon: 'lock', text: t('settings.autoconfig.wizard.access.passwordOnly'), tone: 'plain' };
  else if (keySourceError(a) || keyProblem) access = { icon: 'keys', text: t('settings.autoconfig.editor.factKeysMissing'), tone: 'off' };
  else if (keyCount !== null) {
    const source = forge ?? t('settings.autoconfig.wizard.access.link');
    access = { icon: 'keys', text: t('settings.autoconfig.editor.factKeys', { count: keyCount, source }), tone: 'ok' };
  } else {
    const text = forge ? t('settings.autoconfig.wizard.sum.keysFrom', { forge }) : t('settings.autoconfig.wizard.sum.keysLink');
    access = { icon: 'keys', text, tone: 'ok' };
  }

  // An area left unapplied writes nothing, so it reads as the Armbian default it keeps
  const region: ProfileFact = !m.region.apply
    ? { icon: 'globe', text: t('settings.autoconfig.armbianDefaults'), tone: 'plain' }
    : {
        icon: 'globe',
        text:
          m.region.mode === 'location'
            ? t('settings.autoconfig.wizard.locale.location')
            : m.region.timezone || t('settings.autoconfig.editor.factTzUnset'),
        tone: 'ok',
      };
  const network: ProfileFact = n.apply
    ? { icon: n.mode === 'cable' ? 'cable' : 'wifi', text: netText, tone: 'ok' }
    : { icon: 'wifi', text: t('settings.autoconfig.armbianDefaults'), tone: 'plain' };

  return [
    network,
    {
      icon: 'user',
      text: userNow ? t('settings.autoconfig.editor.factUser', { name: m.user.name.trim() }) : t('settings.autoconfig.wizard.user.later'),
      tone: userNow ? 'ok' : 'plain',
    },
    access,
    region,
  ];
}

export interface ProfileSpec extends ProfileFact {
  /** i18n key of the area's label */
  label: string;
}

/** A saved profile's first-boot facts for the list card, in reading order; under its label the user is just the account */
export function profileSpec(profile: AutoconfigProfile, t: TFn): ProfileSpec[] {
  const model = toEditorModel(profile, { timezone: '', locale: '' });
  const [network, user, access, region] = profileFacts(model, null, false, t);
  const account = model.user.mode === 'now' ? firstLoginUserName(model.user.name) : '';
  return [
    { ...network, label: 'settings.autoconfig.wizard.steps.network' },
    { ...user, text: account || user.text, label: 'settings.autoconfig.wizard.steps.user' },
    { ...region, label: 'settings.autoconfig.wizard.steps.locale' },
    { ...access, label: 'settings.autoconfig.wizard.steps.access' },
  ];
}

export function profileScopeLabel(boards: readonly ProfileBoard[], t: TFn): string {
  if (boards.length === 0) return t('settings.profiles.allBoards');
  if (boards.length === 1) return boards[0].name;
  return t('settings.profiles.boardCount', { count: boards.length });
}
