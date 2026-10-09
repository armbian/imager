// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../../config';
import { useMotion } from '../../../contexts/MotionContext';

interface ActionPillProps {
  isNew: boolean;
  dirty: boolean;
  /** i18n key of why the profile cannot be saved (or created) yet, the first blocking field; null when it can */
  blockedReason: string | null;
  saving: boolean;
  onSave: () => void;
  onDiscard: () => void;
  onCancel: () => void;
}

type StatusTone = 'warn' | 'attn' | 'ok';

/** A blocked primary stays focusable (aria-disabled) so pressing it can scroll to the field that needs a value */
export function ActionPill({ isNew, dirty, blockedReason, saving, onSave, onDiscard, onCancel }: ActionPillProps) {
  const { t } = useTranslation();
  const { reduced } = useMotion();
  const visible = isNew || dirty;

  // Stays mounted through the slide-down, then unmounts
  const [present, setPresent] = useState(visible);
  const [lastVisible, setLastVisible] = useState(visible);
  if (visible !== lastVisible) {
    setLastVisible(visible);
    if (visible) setPresent(true);
  }
  useEffect(() => {
    if (visible || !present) return;
    const timer = setTimeout(() => setPresent(false), reduced ? 0 : UI.ACTION_BAR.OUT_MS);
    return () => clearTimeout(timer);
  }, [visible, present, reduced]);

  if (!present) return null;

  // While it slides down after Discard it keeps the dirty face it had, so nothing changes under the motion
  const [tone, status]: [StatusTone, string] = blockedReason
    ? ['warn', t(blockedReason)]
    : isNew
      ? ['ok', t('settings.autoconfig.editor.readyToCreate')]
      : ['attn', t('settings.autoconfig.editor.unsavedChanges')];

  return (
    <div
      className={`pe-pill ${visible ? 'is-in' : 'is-out'}`}
      role="region"
      aria-label={t('settings.autoconfig.editor.profileActions')}
      inert={!visible || undefined}
      aria-hidden={!visible || undefined}
    >
      <span key={status} className={`pe-pill__status is-${tone}`} aria-live="polite">
        <span className="pe-pill__dot" aria-hidden="true" />
        <span className="pe-pill__text">{status}</span>
      </span>
      <span className="pe-pill__actions">
        <button type="button" className="pe-pill__btn is-ghost" disabled={saving} onClick={isNew ? onCancel : onDiscard}>
          {t(isNew ? 'settings.autoconfig.cancel' : 'settings.autoconfig.editor.discard')}
        </button>
        <button type="button" className="pe-pill__btn is-primary" aria-disabled={!!blockedReason || saving} onClick={onSave}>
          {saving ? t('settings.autoconfig.saving') : t(isNew ? 'settings.autoconfig.wizard.create' : 'settings.autoconfig.save')}
        </button>
      </span>
    </div>
  );
}
