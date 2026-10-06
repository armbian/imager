// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

/** Image filtering predicates and filter button definitions shared by image selection UIs */

import { Star, Shield, RefreshCw, AppWindow, Box, Monitor } from 'lucide-react';
import { IMAGE_STABILITY, type ImageInfo } from '../types';
import { DESKTOP_ENVIRONMENTS } from './badges';
import { PALETTE } from './constants';

/** Trunk / rolling-release builds carry "trunk" in their release version */
export const isTrunkImage = (image: ImageInfo): boolean => image.release.toLowerCase().includes('trunk');

/** Predicates shared by availability checks and filtering. Consumers pick the subset they need. */
export const IMAGE_FILTER_PREDICATES: Record<string, (img: ImageInfo) => boolean> = {
  recommended: (img) => img.promoted === true,
  // Exclude trunk so Stable and Rolling stay mutually exclusive
  stable: (img) => img.stability === IMAGE_STABILITY.STABLE && !isTrunkImage(img),
  rolling: isTrunkImage,
  apps: (img) => !!(img.preinstalled_application && img.preinstalled_application.length > 0),
  // Minimal: no desktop environment and no preinstalled apps
  barebone: (img) => {
    const variant = img.image_variant.toLowerCase();
    const hasDesktop = DESKTOP_ENVIRONMENTS.some((de) => variant.includes(de));
    const hasApp = img.preinstalled_application && img.preinstalled_application.length > 0;
    return !hasDesktop && !hasApp;
  },
};

/** Data-driven filter button list with translation keys and lucide icons */
export const FILTER_BUTTONS: Array<{
  key: string;
  labelKey: string;
  icon: typeof Star;
}> = [
  { key: 'recommended', labelKey: 'modal.promoted', icon: Star },
  { key: 'stable', labelKey: 'modal.stable', icon: Shield },
  { key: 'rolling', labelKey: 'modal.rolling', icon: RefreshCw },
  { key: 'apps', labelKey: 'modal.apps', icon: AppWindow },
  { key: 'barebone', labelKey: 'modal.minimal', icon: Box },
];

/** Category an image belongs to when grouping the "All Images" view. */
export type OsCategory = 'desktop' | 'minimal' | 'apps' | 'rolling';

/** Section headers of the "All Images" view, in display order */
export const OS_CATEGORY_GROUPS: Array<{
  id: OsCategory;
  titleKey: string;
  icon: typeof Star;
  color: string;
}> = [
  { id: 'desktop', titleKey: 'modal.groupDesktop', icon: Monitor, color: PALETTE.BLUE },
  { id: 'minimal', titleKey: 'modal.groupMinimal', icon: Box, color: PALETTE.TEAL },
  { id: 'apps', titleKey: 'modal.groupApps', icon: AppWindow, color: PALETTE.VIOLET },
  { id: 'rolling', titleKey: 'modal.groupRolling', icon: RefreshCw, color: PALETTE.AMBER },
];

/** Dot colour of a build's stability status on recommended cards */
export const IMAGE_STATUS_COLOR = {
  ROLLING: PALETTE.BLUE,
  STABLE: PALETTE.GREEN,
} as const;

export const PROMOTED_COLOR = PALETTE.ORANGE;

/** Assign an image to one category by precedence (trunk, app, desktop, else minimal) so groups never overlap. */
export function categoryOf(img: ImageInfo): OsCategory {
  if (isTrunkImage(img)) return 'rolling';
  if (img.preinstalled_application && img.preinstalled_application.length > 0) return 'apps';
  const variant = img.image_variant.toLowerCase();
  if (DESKTOP_ENVIRONMENTS.some((de) => variant.includes(de))) return 'desktop';
  return 'minimal';
}
