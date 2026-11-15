// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { AutoconfigConfig, AutoconfigProfile, ProfileBoard, UserShell } from '../types';
import { isHttpUrl, trimmedOrUndefined } from '../utils';
import type { TFn } from '../utils/errorUtils';
import type { StaticIpErrors } from '../utils';
import { AUTOCONFIG, UI } from './constants';
import {
  AUTOCONFIG_PLACEHOLDERS,
  RESERVED_USER_NAMES,
  SSH_KEY_FORGES,
  forgeKeysUrl,
  hasNetplanUnsafeChar,
  isValidKeyUser,
  isWifiKeyLength,
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

// The raw name armbian-firstlogin 26.8 accepts; master also allows - and _, so this is the common subset.
const USER_NAME_PATTERN = /^[a-zA-Z][a-zA-Z0-9]*$/;

/** The account armbian-firstlogin creates from a valid name */
export function firstLoginUserName(name: string): string {
  return name.trim().toLowerCase();
}

export type UserNameIssue = 'required' | 'pattern' | 'tooLong' | 'reserved';

export const USER_NAME_ISSUE_KEY: Record<UserNameIssue, string> = {
  required: 'settings.autoconfig.wizard.usernameRequired',
  pattern: 'settings.autoconfig.editor.userLettersDigits',
  tooLong: 'settings.autoconfig.editor.userTooLong',
  reserved: 'settings.autoconfig.editor.userReserved',
};

export function userNameIssue(name: string): UserNameIssue | null {
  const raw = name.trim();
  if (!raw) return 'required';
  if (!USER_NAME_PATTERN.test(raw)) return 'pattern';
  if (raw.length > AUTOCONFIG.USER_NAME_MAX) return 'tooLong';
  return RESERVED_USER_NAMES.has(raw.toLowerCase()) ? 'reserved' : null;
}

/** i18n key of what is wrong with the full name, or null; ':' would break the passwd entry */
export const realNameIssue = (realName: string): string | null =>
  realName.includes(':') ? 'settings.autoconfig.editor.realNameColon' : null;

/** The full name written when none is typed: the account with its first letter uppercased */
export function defaultRealName(name: string): string {
  const account = firstLoginUserName(name);
  return account ? account[0].toUpperCase() + account.slice(1) : '';
}

/** The empty full name field shows what the board writes when it stays empty */
export const realNamePlaceholder = (name: string): string => defaultRealName(name) || AUTOCONFIG_PLACEHOLDERS.USER_REAL_NAME;

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

export const createsUser = (m: EditorModel): boolean => m.user.mode === 'now' && !!m.user.name.trim();

/** i18n keys of the root password field: left empty, it takes the created user's password, else Armbian asks for it */
export function rootPasswordCopy(m: EditorModel): { hint: string; placeholder: string } {
  return createsUser(m)
    ? { hint: 'settings.autoconfig.editor.rootHintUser', placeholder: 'settings.autoconfig.editor.rootSameAsUser' }
    : { hint: 'settings.autoconfig.editor.rootHint', placeholder: 'settings.autoconfig.editor.rootPlaceholder' };
}
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
    // A saved half-chosen region must show what it misses, so defaults never fill it
    region: {
      apply: regionSet,
      mode: chooses ? 'choose' : 'location',
      locale: c.locale ?? (chooses ? '' : defaults.locale),
      timezone: c.timezone ?? (chooses ? '' : defaults.timezone),
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

/** The static IP fields a fixed address cannot do without, keyed like editorIpErrors */
export function editorIpMissing(n: EditorModel['network']): Pick<StaticIpErrors, 'ip' | 'mask'> {
  if (n.ip !== 'fixed') return {};
  return {
    ip: n.address.trim() ? undefined : 'wizard.addressRequired',
    mask: n.mask.trim() ? undefined : 'editor.maskRequired',
  };
}

const WIFI_BAD_CHARS = 'settings.autoconfig.editor.wifiBadChars';

/** i18n keys of what stops the Wi-Fi name and password from being written */
export function wifiIssues(n: EditorModel['network']): { ssid?: string; key?: string } {
  if (!n.apply || !hasWifi(n.mode)) return {};
  const ssid = !n.ssid.trim()
    ? 'settings.autoconfig.wizard.ssidRequired'
    : hasNetplanUnsafeChar(n.ssid)
      ? WIFI_BAD_CHARS
      : undefined;
  let key: string | undefined;
  // An empty key is an open network
  if (n.key) {
    if (/^\s/.test(n.key)) key = 'settings.autoconfig.editor.wifiKeyLeadingSpace';
    else if (hasNetplanUnsafeChar(n.key)) key = WIFI_BAD_CHARS;
    else if (!isWifiKeyLength(n.key)) key = 'settings.autoconfig.editor.wifiKeyLengthNeeded';
  }
  return { ssid, key };
}

const hasIssue = (issues: object) => Object.values(issues).some(Boolean);

function wifiBlocked(n: EditorModel['network']): boolean {
  return hasIssue(wifiIssues(n));
}

function ipBlocked(n: EditorModel['network']): boolean {
  return n.ip === 'fixed' && (hasIssue(editorIpMissing(n)) || hasIssue(editorIpErrors(n)));
}

function networkBlocked(n: EditorModel['network']): boolean {
  return n.apply && (wifiBlocked(n) || ipBlocked(n));
}

function userBlocked(u: EditorModel['user']): boolean {
  return u.mode === 'now' && (!!userNameIssue(u.name) || !u.password || !!realNameIssue(u.realName));
}

export const REGION_NEEDS_BOTH = 'settings.autoconfig.editor.regionNeedsBoth';

/** "I choose" writes both the language and the time zone, or neither */
export const regionIncomplete = (r: EditorModel['region']): boolean =>
  r.apply && r.mode === 'choose' && !(r.locale && r.timezone);

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
    user: m.user.mode === 'later' ? 'none' : userBlocked(m.user) ? 'attn' : 'set',
    access: m.access.mode === 'password' ? 'none' : keySourceError(m.access) || keyProblem ? 'attn' : 'set',
    region: !m.region.apply ? 'none' : regionIncomplete(m.region) ? 'attn' : 'set',
    advanced: !m.remote.trim() ? 'none' : isHttpUrl(m.remote) ? 'set' : 'attn',
  };
  const done = EDITOR_GROUPS.filter((g) => status[g] === 'set').length;
  return { status, done, total: EDITOR_GROUPS.length };
}

type BlockingGroup = { group: EditorGroup; more?: MoreKey; empty?: boolean };

/** The first group whose fields first boot cannot be given as they are, with the folded "More options" that holds the problem */
function firstBootBlockingGroup(m: EditorModel): BlockingGroup | null {
  if (networkBlocked(m.network)) return { group: 'network', more: wifiBlocked(m.network) ? undefined : 'network' };
  if (userBlocked(m.user)) {
    const inMore = !userNameIssue(m.user.name) && !!m.user.password;
    return { group: 'user', more: inMore ? 'user' : undefined };
  }
  if (keySourceError(m.access)) return { group: 'access' };
  if (regionIncomplete(m.region)) return { group: 'region' };
  if (m.remote.trim() && !isHttpUrl(m.remote)) return { group: 'advanced' };
  return null;
}

/** The first group that stops a save, with the folded "More options" that holds the problem; null when it can save */
export function firstBlockingGroup(m: EditorModel): BlockingGroup | null {
  if (!m.name.trim() || (m.scope === 'some' && !m.boards.length)) return { group: 'name' };
  const blocking = firstBootBlockingGroup(m);
  if (blocking) return blocking;
  // A profile that leaves every area at Armbian's defaults would write an empty preset
  return presetIsEmpty(editorConfig(m)) ? { group: 'network', empty: true } : null;
}

/** i18n key of why the profile cannot be saved (or created) yet, from the first blocking field; null when it can */
export function blockingReason(m: EditorModel, isNew: boolean): string | null {
  const blocking = firstBlockingGroup(m);
  if (!blocking) return null;
  const key = !m.name.trim() ? 'Name' : blocking.group === 'name' ? 'Board' : blocking.empty ? 'Something' : 'Fix';
  if (isNew && key === 'Name') return 'settings.autoconfig.editor.addNameToCreate';
  return `settings.autoconfig.editor.${isNew ? 'create' : 'save'}Needs${key}`;
}

const NO_DEFAULTS = { timezone: '', locale: '' };

/** The group a saved profile must fix before it can be flashed under the current rules, or null */
export function profileNeedsFix(profile: AutoconfigProfile): EditorGroup | null {
  const blocking = firstBlockingGroup(toEditorModel(profile, NO_DEFAULTS));
  return blocking && !blocking.empty ? blocking.group : null;
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

  const userNow = createsUser(m);
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

  const region: ProfileFact = !m.region.apply
    ? { icon: 'globe', text: t('settings.autoconfig.armbianDefaults'), tone: 'plain' }
    : {
        icon: 'globe',
        text:
          m.region.mode === 'location'
            ? t('settings.autoconfig.wizard.locale.thisComputer')
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
      text: userNow ? t('settings.autoconfig.editor.factUser', { name: firstLoginUserName(m.user.name) }) : t('settings.autoconfig.wizard.user.later'),
      tone: userNow ? 'ok' : 'plain',
    },
    access,
    region,
  ];
}

export interface BootVerdict {
  tone: 'ok' | 'warn' | 'plain';
  title: string;
  hint: string | null;
}

/** Whether armbian-firstlogin finishes with this profile or stops at the board's console, and why */
export function firstBootVerdict(m: EditorModel, t: TFn): BootVerdict {
  // Until every first-boot field can be written the outcome is unknown; the name and boards play no part in it
  if (firstBootBlockingGroup(m)) {
    return { tone: 'warn', title: t('settings.autoconfig.editor.verdictNeedsFix'), hint: t('settings.autoconfig.editor.verdictNeedsFixHint') };
  }
  const remote = m.remote.trim();
  if (remote && isHttpUrl(remote)) {
    // The remote config is fetched before the profile's network comes up, so Wi-Fi alone cannot reach it
    const wifiOnly = m.network.apply && m.network.mode === 'wifi';
    return {
      tone: wifiOnly ? 'warn' : 'plain',
      title: t('settings.autoconfig.editor.verdictRemote'),
      hint: t(wifiOnly ? 'settings.autoconfig.editor.verdictRemoteWifi' : 'settings.autoconfig.editor.verdictRemoteHint'),
    };
  }
  if (createsUser(m)) {
    const name = firstLoginUserName(m.user.name);
    return {
      tone: 'ok',
      title: t('settings.autoconfig.editor.verdictFinishes'),
      hint: m.access.rootPassword ? null : t('settings.autoconfig.editor.verdictRootFromUser', { name }),
    };
  }
  return {
    tone: 'warn',
    title: t(m.access.rootPassword ? 'settings.autoconfig.editor.verdictAsksUser' : 'settings.autoconfig.editor.verdictAsksRootAndUser'),
    hint: t('settings.autoconfig.editor.verdictConsoleHint'),
  };
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
