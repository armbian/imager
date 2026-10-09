// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, type ReactNode } from 'react';

interface SettingsGroupProps {
  eyebrow: string;
  attention?: boolean;
  danger?: boolean;
  children: ReactNode;
}

export function SettingsGroup({ eyebrow, attention = false, danger = false, children }: SettingsGroupProps) {
  const headingId = useId();
  return (
    <section className={`set-group${danger ? ' set-group--danger' : ''}`} aria-labelledby={headingId}>
      <h3 id={headingId} className="set-group__eyebrow">
        {eyebrow}
        {attention && <span className="set-group__dot" aria-hidden="true" />}
      </h3>
      <div className="set-group__card">{children}</div>
    </section>
  );
}
