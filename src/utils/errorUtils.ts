// Parse the backend's tagged error strings (e.g. [SHA_UNAVAILABLE]) and map them to i18n keys

import { formatBytes } from './index';

type TFn = (key: string, opts?: Record<string, unknown>) => string;

/** Check if a SHA error indicates the SHA file was unavailable (not a mismatch) */
export function isShaUnavailableError(error: string): boolean {
  return error.includes('[SHA_UNAVAILABLE]');
}

/** Map tagged backend flash errors ([WRITE_FAILED:offset], [QDL_*]) to translated messages */
export function translateFlashError(error: string, t: TFn): string {
  const write = error.match(/\[WRITE_FAILED:(\d+)\]/);
  if (write) return t('error.writeFailed', { offset: formatBytes(Number(write[1])) });
  return translateQdlError(error, t);
}

/** Map QDL backend error tags to translated user-facing messages */
export function translateQdlError(error: string, t: TFn): string {
  if (error.includes('[QDL_DISCONNECTED]')) return t('error.qdlDisconnected');
  if (error.includes('[QDL_CANCELLED]')) return t('error.qdlCancelled');
  if (error.includes('[QDL_PERMISSION_DENIED]')) return t('error.qdlPermissionDenied');
  if (error.includes('[QDL_CONNECTION_FAILED]')) return t('error.qdlConnectionFailed');
  if (error.includes('[QDL_AUTOCONFIG_FAILED]')) return t('error.qdlAutoconfigFailed');
  if (error.includes('[QDL_ERROR]')) return error.replace('[QDL_ERROR] ', '');
  return error;
}
