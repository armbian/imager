// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { CircleCheck, Info, TriangleAlert } from 'lucide-react';
import type { BootVerdict } from '../../../config/profileEditorModel';

const VERDICT_ICON = { ok: CircleCheck, warn: TriangleAlert, plain: Info } as const;

/** How first boot ends with this profile (above the editor header and closing the wizard review) */
export function BootVerdictNote({ verdict }: { verdict: BootVerdict }) {
  const Icon = VERDICT_ICON[verdict.tone];
  return (
    <div className={`pe-verdict is-${verdict.tone}${verdict.hint ? '' : ' is-brief'}`}>
      <Icon size={16} className="pe-verdict__icon" aria-hidden="true" />
      <p className="pe-verdict__text">
        <strong>{verdict.title}</strong>
        {verdict.hint && <span>{verdict.hint}</span>}
      </p>
    </div>
  );
}
