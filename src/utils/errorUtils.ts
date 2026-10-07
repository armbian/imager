// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

// Parse the backend's tagged error strings (e.g. [SHA_UNAVAILABLE]) and map them to i18n keys

import { formatBytes } from './index';

export type TFn = (key: string, opts?: Record<string, unknown>) => string;

const WRITE_FAILED_PATTERN = /\[WRITE_FAILED:(\d+)\]/;
const DEVICE_TOO_SMALL_PATTERN = /\[DEVICE_TOO_SMALL:(\d+):(\d+)\]/;
const QDL_ERROR_TAG = '[QDL_ERROR]';
/** Twin of autoconfig::TAG_NOT_ARMBIAN in src-tauri/src/autoconfig.rs */
const AUTOCONFIG_NOT_ARMBIAN_TAG = '[AUTOCONFIG_NOT_ARMBIAN]';

const DEVICE_ERROR_KEYS = {
  '[DEVICE_INVALID_PATH]': 'error.deviceInvalidPath',
  '[DEVICE_NOT_FOUND]': 'error.deviceNotFound',
  '[DEVICE_CHANGED]': 'error.deviceChanged',
  '[DEVICE_READ_ONLY]': 'error.deviceReadOnly',
  '[DEVICE_SYSTEM_BLOCKED]': 'error.deviceSystemBlocked',
  // Twin of TAG_FLASH_BUSY in src-tauri/src/utils/errors.rs: the previous flash still holds the device
  '[FLASH_BUSY]': 'error.flashBusy',
} as const;

/** Twins of TAG_SHA_MISMATCH (download.rs), TAG_VERIFY_* (flash/verify.rs), TAG_DECOMPRESS_FAILED (decompress.rs) and TAG_CANCELLED (utils/errors.rs) */
const IMAGE_ERROR_KEYS = {
  // Only an EDL flash keeps its screen after Cancel, so a cancelled download or decompress lands here
  '[CANCELLED]': 'error.qdlCancelled',
  '[SHA_MISMATCH]': 'error.shaMismatch',
  '[VERIFY_MISMATCH]': 'error.verifyMismatch',
  '[VERIFY_READ_FAILED]': 'error.verifyReadFailed',
  '[DECOMPRESS_FAILED]': 'error.imageDecompressFailed',
} as const;

const QDL_ERROR_KEYS = {
  '[QDL_DISCONNECTED]': 'error.qdlDisconnected',
  '[QDL_CANCELLED]': 'error.qdlCancelled',
  '[QDL_PERMISSION_DENIED]': 'error.qdlPermissionDenied',
  '[QDL_CONNECTION_FAILED]': 'error.qdlConnectionFailed',
  '[QDL_AUTOCONFIG_FAILED]': 'error.qdlAutoconfigFailed',
  '[QDL_DEVICE_NOT_FOUND]': 'error.qdlDeviceNotFound',
  '[QDL_MULTIPLE_DEVICES]': 'error.qdlMultipleDevices',
} as const;

function findTaggedKey(error: string, keys: Record<string, string>): string | undefined {
  return Object.entries(keys).find(([tag]) => error.includes(tag))?.[1];
}

/** Check if a SHA error indicates the SHA file was unavailable (not a mismatch) */
export function isShaUnavailableError(error: string): boolean {
  return error.includes('[SHA_UNAVAILABLE]');
}

/** True when the backend refused the target device before writing ([DEVICE_*] tags, [FLASH_BUSY]) */
export function isDeviceRefusalError(error: string): boolean {
  return DEVICE_TOO_SMALL_PATTERN.test(error) || findTaggedKey(error, DEVICE_ERROR_KEYS) !== undefined;
}

/** Map tagged backend flash errors ([AUTOCONFIG_NOT_ARMBIAN], [DEVICE_*], [FLASH_BUSY], [WRITE_FAILED:offset], image checks, [QDL_*]) to translated messages */
export function translateFlashError(error: string, t: TFn): string {
  // Block and QDL flashes both refuse a profile this way; the UFS path wraps it in [QDL_AUTOCONFIG_FAILED].
  if (error.includes(AUTOCONFIG_NOT_ARMBIAN_TAG)) return t('error.autoconfigNotArmbian');
  const tooSmall = error.match(DEVICE_TOO_SMALL_PATTERN);
  if (tooSmall) {
    return t('error.deviceTooSmall', {
      needed: formatBytes(Number(tooSmall[1])),
      available: formatBytes(Number(tooSmall[2])),
    });
  }
  const deviceKey = findTaggedKey(error, DEVICE_ERROR_KEYS);
  if (deviceKey) return t(deviceKey);
  const write = error.match(WRITE_FAILED_PATTERN);
  if (write) return t('error.writeFailed', { offset: formatBytes(Number(write[1])) });
  const imageKey = findTaggedKey(error, IMAGE_ERROR_KEYS);
  if (imageKey) return t(imageKey);
  return translateQdlError(error, t);
}

/** Map QDL backend error tags to translated user-facing messages */
export function translateQdlError(error: string, t: TFn): string {
  const key = findTaggedKey(error, QDL_ERROR_KEYS);
  if (key) return t(key);
  if (error.includes(QDL_ERROR_TAG)) return error.replace(`${QDL_ERROR_TAG} `, '');
  return error;
}

/** Key lookup failures the person can act on, worded for a forge account or for a link */
const SSH_KEYS_ERROR_KEYS = {
  '[SSH_KEYS_NOT_FOUND]': { forge: 'settings.autoconfig.keys.notFoundForge', link: 'settings.autoconfig.keys.notFoundLink' },
  '[SSH_KEYS_NONE]': { forge: 'settings.autoconfig.keys.noneForge', link: 'settings.autoconfig.keys.noneLink' },
  '[SSH_KEYS_TOO_LARGE]': { forge: 'settings.autoconfig.keys.tooLarge', link: 'settings.autoconfig.keys.tooLarge' },
  '[SSH_KEYS_INVALID_INPUT]': { forge: 'settings.autoconfig.keys.invalidForge', link: 'settings.autoconfig.keys.invalidLink' },
} as const;

function findSshKeysError(error: string) {
  return Object.entries(SSH_KEYS_ERROR_KEYS).find(([tag]) => error.includes(tag))?.[1];
}

/** True when the key source itself looks wrong; anything else ([SSH_KEYS_NETWORK] included) only means it could not be checked */
export function isSshKeysSourceError(error: string): boolean {
  return findSshKeysError(error) !== undefined;
}

/** Map an [SSH_KEYS_*] lookup error to a hint; `forge` is the brand label, or null for a link. Never shown verbatim. */
export function translateSshKeysError(error: string, t: TFn, forge: string | null): string {
  const keys = findSshKeysError(error);
  if (!keys) return t('settings.autoconfig.keys.unreachable');
  return forge ? t(keys.forge, { forge }) : t(keys.link);
}
