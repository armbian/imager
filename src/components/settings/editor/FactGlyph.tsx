// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { Cable, Globe, KeyRound, Lock, User, Wifi } from 'lucide-react';
import type { FactIcon } from '../../../config/profileEditorModel';

const FACT_ICON: Record<FactIcon, typeof Cable> = { cable: Cable, wifi: Wifi, user: User, keys: KeyRound, lock: Lock, globe: Globe };

/** The glyph of a first-boot fact (editor header and profile cards) */
export function FactGlyph({ icon, size }: { icon: FactIcon; size: number }) {
  const Icon = FACT_ICON[icon];
  return <Icon size={size} aria-hidden="true" />;
}
