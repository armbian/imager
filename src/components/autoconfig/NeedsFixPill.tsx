// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useTranslation } from 'react-i18next';
import { TriangleAlert } from 'lucide-react';

/** A saved profile the current rules refuse: it cannot be flashed until it is edited */
export function NeedsFixPill() {
  const { t } = useTranslation();
  return (
    <span className="profile-fix">
      <TriangleAlert size={12} aria-hidden="true" />
      {t('settings.profiles.needsFix')}
    </span>
  );
}
