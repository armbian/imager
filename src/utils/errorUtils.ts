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

/** True when the backend refused the target device before writing ([DEVICE_*] tags) */
export function isDeviceRefusalError(error: string): boolean {
  return DEVICE_TOO_SMALL_PATTERN.test(error) || findTaggedKey(error, DEVICE_ERROR_KEYS) !== undefined;
}

/** Map tagged backend flash errors ([AUTOCONFIG_NOT_ARMBIAN], [DEVICE_*], [WRITE_FAILED:offset], [QDL_*]) to translated messages */
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
  return translateQdlError(error, t);
}

/** Map QDL backend error tags to translated user-facing messages */
export function translateQdlError(error: string, t: TFn): string {
  const key = findTaggedKey(error, QDL_ERROR_KEYS);
  if (key) return t(key);
  if (error.includes(QDL_ERROR_TAG)) return error.replace(`${QDL_ERROR_TAG} `, '');
  return error;
}
