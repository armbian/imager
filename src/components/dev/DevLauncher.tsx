// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { Ref } from 'react';
import { FlaskConical } from 'lucide-react';
import { DEV_SCENARIOS } from '../../config/devScenarios';

interface DevLauncherProps {
  open: boolean;
  active: boolean;
  shortcut: string;
  onToggle: () => void;
  ref?: Ref<HTMLButtonElement>;
}

export function DevLauncher({ open, active, shortcut, onToggle, ref }: DevLauncherProps) {
  return (
    <button
      ref={ref}
      type="button"
      className={`dev-launcher${active ? ' is-active' : ''}`}
      aria-expanded={open}
      aria-controls={DEV_SCENARIOS.DRAWER_ID}
      aria-label={`${DEV_SCENARIOS.TITLE}${active ? ', simulation active' : ''} (${shortcut})`}
      onClick={onToggle}
    >
      <FlaskConical size={DEV_SCENARIOS.ICON_SIZE.SM} aria-hidden="true" className="dev-launcher__icon" />
      <span className={active ? 'dev-launcher__pill' : 'dev-launcher__label'} aria-hidden="true">
        {active ? 'Simulated' : DEV_SCENARIOS.TITLE}
      </span>
    </button>
  );
}
