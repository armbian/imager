// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import type { Ref } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings } from 'lucide-react';

interface SettingsButtonProps {
  active: boolean;
  onClick: () => void;
  ref?: Ref<HTMLButtonElement>;
}

/** Header gear: opens the settings page, closes it again while it shows. */
export function SettingsButton({ active, onClick, ref }: SettingsButtonProps) {
  const { t } = useTranslation();

  return (
    <button
      ref={ref}
      type="button"
      className={`header-gear${active ? ' is-active' : ''}`}
      onClick={onClick}
      aria-label={t('settings.title')}
      aria-pressed={active}
    >
      <Settings size={20} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}
