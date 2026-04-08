/**
 * Backend error tag parsing and translation utilities
 *
 * The Rust backend uses tagged error strings (e.g., [SHA_UNAVAILABLE], [QDL_DISCONNECTED])
 * to communicate error types. These functions parse the tags and map them to i18n keys.
 */

/** Check if a SHA error indicates the SHA file was unavailable (not a mismatch) */
export function isShaUnavailableError(error: string): boolean {
  return error.includes('[SHA_UNAVAILABLE]');
}

/** Map QDL backend error tags to translated user-facing messages */
export function translateQdlError(error: string, t: (key: string) => string): string {
  if (error.includes('[QDL_DISCONNECTED]')) return t('error.qdlDisconnected');
  if (error.includes('[QDL_CANCELLED]')) return t('error.qdlCancelled');
  if (error.includes('[QDL_ERROR]')) return error.replace('[QDL_ERROR] ', '');
  return error;
}
