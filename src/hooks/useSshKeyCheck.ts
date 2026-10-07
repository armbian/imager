// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState } from 'react';
import { AUTOCONFIG } from '../config';
import { getErrorMessage } from '../utils';
import { useAsyncData } from './useAsyncData';
import { lookupSshKeys } from './useTauri';
import type { SshKeyLookup, SshKeySource } from '../types';

export type SshKeyCheck =
  | { status: 'idle' }
  | { status: 'checking' }
  | { status: 'found'; lookup: SshKeyLookup }
  | { status: 'failed'; error: string };

interface Query {
  key: string;
  source: SshKeySource;
  value: string;
}

type Answer = { key: string } & ({ lookup: SshKeyLookup } | { error: string });

const IDLE: SshKeyCheck = { status: 'idle' };
const CHECKING: SshKeyCheck = { status: 'checking' };

/** Looks up the public keys behind a key source once its input settles; `enabled` only once the format check passes. */
export function useSshKeyCheck(source: SshKeySource, value: string, enabled: boolean): SshKeyCheck {
  const trimmed = value.trim();
  const query: Query | null = enabled && trimmed ? { key: `${source}\x1f${trimmed}`, source, value: trimmed } : null;
  const queryKey = query?.key ?? '';
  const [settled, setSettled] = useState<Query | null>(query);

  useEffect(() => {
    const id = window.setTimeout(
      () => setSettled(queryKey ? { key: queryKey, source, value: trimmed } : null),
      AUTOCONFIG.KEY_LOOKUP_DEBOUNCE_MS
    );
    return () => window.clearTimeout(id);
  }, [queryKey, source, trimmed]);

  // Each result carries the query it answers, so a reply for an older input is never shown as current.
  const { data } = useAsyncData(
    async (): Promise<Answer | null> => {
      if (!settled) return null;
      try {
        return { key: settled.key, lookup: await lookupSshKeys(settled.source, settled.value) };
      } catch (err) {
        return { key: settled.key, error: getErrorMessage(err) };
      }
    },
    [settled?.key],
    { immediate: !!settled }
  );

  if (!query) return IDLE;
  if (!data || data.key !== query.key) return CHECKING;
  return 'lookup' in data ? { status: 'found', lookup: data.lookup } : { status: 'failed', error: data.error };
}
