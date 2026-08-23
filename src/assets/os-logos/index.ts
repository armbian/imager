// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Armbian and Armbian Imager contributors

import { OS_INFO } from '../../config/os-info';

/** Resolve a human-readable OS name from a distro release string. */
export function getOsName(distroRelease: string): string {
  const distro = distroRelease.toLowerCase();

  for (const [codename, info] of Object.entries(OS_INFO)) {
    if (distro.includes(codename)) {
      return info.name;
    }
  }

  if (distro.includes('ubuntu')) {
    return 'Ubuntu';
  }

  if (distro.includes('debian')) {
    return 'Debian';
  }

  return 'Armbian';
}
