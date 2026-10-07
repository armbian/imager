// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

// Only a hint: the board fetches the keys itself at first boot, so no result here ever blocks saving.

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { CircleCheck, CloudOff, Loader2, TriangleAlert } from 'lucide-react';
import { AUTOCONFIG, UI } from '../../config';
import { shortFingerprint } from '../../config/autoconfig';
import { useSshKeyCheck, type SshKeyCheck } from '../../hooks/useSshKeyCheck';
import { isSshKeysSourceError, translateSshKeysError } from '../../utils/errorUtils';
import { isHttpUrl } from '../../utils';
import { MarqueeText } from '../shared/MarqueeText';

const ICON = UI.ICON_SIZE.WIZARD_HINT;

interface SshKeyStatusProps {
  check: SshKeyCheck;
  forge: string | null;
  showKeys?: boolean;
}

export function SshKeyStatus({ check, forge, showKeys = false }: SshKeyStatusProps) {
  const { t } = useTranslation();
  if (check.status === 'idle') return null;

  let tone: 'ok' | 'warn' | 'soft';
  let icon: ReactNode;
  let text: string;
  switch (check.status) {
    case 'checking':
      tone = 'soft';
      icon = <Loader2 size={ICON} className="spinning" aria-hidden="true" />;
      text = t('settings.autoconfig.keys.checking');
      break;
    case 'found':
      tone = 'ok';
      icon = <CircleCheck size={ICON} aria-hidden="true" />;
      text = t('settings.autoconfig.keys.found', { count: check.lookup.total });
      break;
    case 'failed': {
      const sourceError = isSshKeysSourceError(check.error);
      tone = sourceError ? 'warn' : 'soft';
      icon = sourceError ? <TriangleAlert size={ICON} aria-hidden="true" /> : <CloudOff size={ICON} aria-hidden="true" />;
      text = translateSshKeysError(check.error, t, forge);
      break;
    }
  }

  const keys = check.status === 'found' && showKeys ? check.lookup.keys.slice(0, AUTOCONFIG.KEY_LOOKUP_VISIBLE_ROWS) : [];
  const more = check.status === 'found' ? check.lookup.total - keys.length : 0;

  return (
    <div className="ssh-check">
      <div className={`ssh-check__line is-${tone}`} role="status">
        {icon}
        <span>{text}</span>
      </div>
      {keys.length > 0 && (
        <ul className="ssh-keys">
          {keys.map((key, i) => (
            <li key={i} className="ssh-keys__row">
              <span className="ssh-keys__type">{key.label}</span>
              <MarqueeText
                className={`ssh-keys__comment${key.comment ? '' : ' is-empty'}`}
                text={key.comment ?? t('settings.autoconfig.keys.noComment')}
              />
              <span className="ssh-keys__fp" title={key.fingerprint}>{shortFingerprint(key.fingerprint)}</span>
            </li>
          ))}
          {more > 0 && <li className="ssh-keys__more">{t('settings.autoconfig.keys.more', { count: more })}</li>}
        </ul>
      )}
    </div>
  );
}

export function SshKeyUrlCheck({ url }: { url: string | undefined }) {
  const value = url ?? '';
  const check = useSshKeyCheck('url', value, isHttpUrl(value));
  return <SshKeyStatus check={check} forge={null} />;
}
