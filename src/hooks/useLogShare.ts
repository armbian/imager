// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useRef, useState } from 'react';
import { useLogUpload } from './useLogUpload';

/** Uploads the session log once and opens the share modal; later calls reopen it with the same link. */
export function useLogShare() {
  const { upload, uploading, url, qr, error, open } = useLogUpload();
  const [isOpen, setIsOpen] = useState(false);
  // The trigger is disabled while uploading, so the dialog cannot rely on the focused element to hand focus back.
  const triggerRef = useRef<HTMLButtonElement>(null);

  const share = useCallback(async () => {
    if (uploading) return;
    if (url || (await upload())) setIsOpen(true);
  }, [uploading, url, upload]);

  const onClose = useCallback(() => setIsOpen(false), []);

  return {
    share,
    uploading,
    uploaded: url !== null,
    error,
    triggerRef,
    modalProps: { isOpen, onClose, url, qr, onOpen: open, returnFocusRef: triggerRef },
  };
}
