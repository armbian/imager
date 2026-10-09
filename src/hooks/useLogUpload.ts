// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getErrorMessage } from '../utils';
import { uploadLogs } from './useTauri';

interface LogUpload {
  /** Resolves to the paste URL, or null when the upload failed (see error) */
  upload: () => Promise<string | null>;
  uploading: boolean;
  url: string | null;
  error: string | null;
}

/** Uploads the session log to the paste service and keeps the resulting link. */
export function useLogUpload(): LogUpload {
  const { t } = useTranslation();
  const [uploading, setUploading] = useState(false);
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const upload = useCallback(async () => {
    setUploading(true);
    setError(null);
    try {
      const result = await uploadLogs();
      setUrl(result.url);
      return result.url;
    } catch (err) {
      setError(getErrorMessage(err, t('error.uploadFailed')));
      return null;
    } finally {
      setUploading(false);
    }
  }, [t]);

  return { upload, uploading, url, error };
}
