// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import {
  Lightbulb,
  Newspaper,
  Heart,
  GitPullRequest,
  LifeBuoy,
  Hammer,
  type LucideIcon,
} from 'lucide-react';

export interface MotdTip {
  message: string;
  url: string;
  type: string;
}

const MOTD_EXCLUDED_TYPES = ['software'] as const;

const MOTD_TITLE_TYPES = ['news'] as const;

const MOTD_ICONS: Record<string, LucideIcon> = {
  news: Newspaper,
  sponsor: Heart,
  contribute: GitPullRequest,
  service: LifeBuoy,
  build: Hammer,
};

export function motdIconFor(type: string): LucideIcon {
  return Object.hasOwn(MOTD_ICONS, type) ? MOTD_ICONS[type] : Lightbulb;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function nonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function isExcludedType(type: unknown): boolean {
  return (MOTD_EXCLUDED_TYPES as readonly unknown[]).includes(type);
}

function usesTitle(type: unknown): boolean {
  return (MOTD_TITLE_TYPES as readonly unknown[]).includes(type);
}

function isActive(expires: unknown, now: Date): boolean {
  if (!nonEmptyString(expires)) return true;
  const expDate = new Date(expires);
  return isNaN(expDate.getTime()) || expDate > now;
}

export function featuredToMotdTips(feed: unknown, now: Date): MotdTip[] {
  if (!isRecord(feed) || !Array.isArray(feed.entries)) return [];

  const tips: MotdTip[] = [];
  for (const entry of feed.entries as unknown[]) {
    if (!isRecord(entry) || isExcludedType(entry.type)) continue;
    const line = isRecord(entry.motd) ? entry.motd.line : undefined;
    const message = usesTitle(entry.type) && nonEmptyString(entry.title) ? entry.title : line;
    if (!nonEmptyString(message) || !nonEmptyString(entry.url)) continue;
    if (!isActive(entry.expires, now)) continue;
    tips.push({ message, url: entry.url, type: typeof entry.type === 'string' ? entry.type : '' });
  }
  return tips;
}
