// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import QRCode from 'qrcode';
import { COLORS, QR_CODE } from '../config';
import { getErrorMessage } from '../utils';
import { openExternal, uploadLogs } from './useTauri';

interface LogUpload {
  /** Resolves to the paste URL, or null when the upload failed (see error) */
  upload: () => Promise<string | null>;
  uploading: boolean;
  url: string | null;
  /** QR code of the paste URL as a data URL, only with `withQr` */
  qr: string | null;
  error: string | null;
  open: () => Promise<void>;
}

/** Uploads the session log to the paste service and keeps the resulting link. */
export function useLogUpload({ withQr = false }: { withQr?: boolean } = {}): LogUpload {
  const { t } = useTranslation();
  const [uploading, setUploading] = useState(false);
  const [url, setUrl] = useState<string | null>(null);
  const [qr, setQr] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const upload = useCallback(async () => {
    setUploading(true);
    setError(null);
    let pasteUrl: string;
    try {
      pasteUrl = (await uploadLogs()).url;
      setUrl(pasteUrl);
    } catch (err) {
      setError(getErrorMessage(err, t('error.uploadFailed')));
      setUploading(false);
      return null;
    }
    if (withQr) {
      try {
        setQr(await QRCode.toDataURL(pasteUrl, {
          width: QR_CODE.WIDTH,
          margin: QR_CODE.MARGIN,
          color: { dark: COLORS.QR_DARK, light: COLORS.QR_LIGHT },
        }));
      } catch (err) {
        setError(getErrorMessage(err, t('error.uploadFailed')));
      }
    }
    setUploading(false);
    return pasteUrl;
  }, [t, withQr]);

  const open = useCallback(async () => {
    if (url) await openExternal(url);
  }, [url]);

  return { upload, uploading, url, qr, error, open };
}
