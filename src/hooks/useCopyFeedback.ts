// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useRef, useState } from 'react';
import { TIMING } from '../config';

/** `copy` rejects when the clipboard write fails, so each caller keeps its own error handling. */
export function useCopyFeedback(text: string | null | undefined) {
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => {
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);

  const copy = useCallback(async () => {
    if (!text) return;
    await navigator.clipboard.writeText(text);
    setCopied(true);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setCopied(false), TIMING.COPIED_NOTIFICATION);
  }, [text]);

  return { copied, copy };
}
