// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { LucideIcon } from 'lucide-react';
import { UI } from '../../config';

interface EmptyStateAction {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
}

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  hint?: string;
  action?: EmptyStateAction;
}

export function EmptyState({ icon: Icon, title, hint, action }: EmptyStateProps) {
  return (
    <div className="device-empty">
      <span className="device-empty__icon" aria-hidden="true">
        <Icon size={UI.ICON_SIZE.EMPTY_STATE} />
      </span>
      <p className="device-empty__title">{title}</p>
      {hint && <p className="device-empty__hint">{hint}</p>}
      {action && (
        <button type="button" className="device-refresh-btn" onClick={action.onClick}>
          <action.icon size={UI.ICON_SIZE.EMPTY_STATE_ACTION} />
          {action.label}
        </button>
      )}
    </div>
  );
}
