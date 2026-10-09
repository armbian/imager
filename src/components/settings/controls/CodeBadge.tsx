// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { ReactNode } from 'react';

export function CodeBadge({ children }: { children: ReactNode }) {
  return <span className="code-badge">{children}</span>;
}
