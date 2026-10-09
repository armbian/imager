// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useMemo, useState } from 'react';
import { PARTNER_TIER_RANK, SUPPORT_TIER_ORDER, UI } from '../config';
import type { AutoconfigProfile, BoardInfo, ProfileBoard, VendorInfo } from '../types';
import { bestMatchRank, compareBoardsBySupport, matchRank } from '../utils';
import { useAsyncData } from './useAsyncData';
import { useBoardPhotos } from './useBoardPhotos';
import { getAutoconfigProfiles } from './useSettings';
import { useSkeletonLoading } from './useSkeletonLoading';
import { getBoards, getVendors } from './useTauri';
import { useVendorLogos } from './useVendorLogos';

export interface BoardVendor {
  id: string;
  name: string;
  logo: string | null;
  /** Partner tier (platinum, gold, silver) or null */
  tier: string | null;
  /** The brand's boards in catalog order */
  boards: BoardInfo[];
}

/** A picked board with its catalog entry, absent when the catalog no longer lists it */
export interface PickedBoard {
  slug: string;
  name: string;
  board: BoardInfo | null;
}

export type ReviewFilter = { kind: 'tier' | 'vendor'; value: string } | null;

type BoardsUpdate = (update: (boards: ProfileBoard[]) => ProfileBoard[]) => void;

const byName = (a: { name: string }, b: { name: string }) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
const tierIndex = (tier: string | undefined) => {
  const at = tier ? SUPPORT_TIER_ORDER.indexOf(tier) : -1;
  return at < 0 ? SUPPORT_TIER_ORDER.length : at;
};
const toProfileBoard = (b: BoardInfo): ProfileBoard => ({ slug: b.slug, name: b.name });

/** Ranked brands whose name contains the needle: prefix first, then word start, then the bigger brand. */
export function rankVendors(vendors: readonly BoardVendor[], needle: string): BoardVendor[] {
  return vendors
    .map((v) => ({ v, rank: matchRank(v.name, needle) }))
    .filter((x) => x.rank >= 0)
    .sort((a, b) => a.rank - b.rank || b.v.boards.length - a.v.boards.length || byName(a.v, b.v))
    .map((x) => x.v);
}

/** The board catalog with its brands, partner tiers, logos and the brands used by the user's other profiles. */
function useBoardCatalog(profileId: string | undefined) {
  const { data: boards, loading, error, reload } = useAsyncData<BoardInfo[]>(() => getBoards(), []);
  const { data: vendorInfo } = useAsyncData<VendorInfo[]>(() => getVendors().catch(() => []), []);
  const { data: profiles } = useAsyncData<AutoconfigProfile[]>(() => getAutoconfigProfiles().catch(() => []), []);
  const { cachedUrls: logos } = useVendorLogos(boards, true);
  const { showSkeleton } = useSkeletonLoading(loading, !!boards || !loading);

  const ordered = useMemo(() => [...(boards ?? [])].sort(compareBoardsBySupport), [boards]);
  const bySlug = useMemo(() => new Map(ordered.map((b) => [b.slug, b])), [ordered]);

  const vendors = useMemo(() => {
    const tiers = new Map<string, string>();
    for (const v of vendorInfo ?? []) {
      const tier = v.partner_tier?.toLowerCase();
      if (tier && tier in PARTNER_TIER_RANK) tiers.set(v.slug, tier);
    }
    const byId = new Map<string, BoardVendor>();
    for (const b of ordered) {
      if (!b.vendor) continue;
      const v = byId.get(b.vendor) ?? {
        id: b.vendor,
        name: b.vendor_name || b.vendor,
        logo: logos.get(b.vendor) ?? null,
        tier: tiers.get(b.vendor) ?? null,
        boards: [],
      };
      v.boards.push(b);
      byId.set(b.vendor, v);
    }
    return [...byId.values()].sort(byName);
  }, [ordered, vendorInfo, logos]);

  const vendorById = useMemo(() => new Map(vendors.map((v) => [v.id, v])), [vendors]);
  const partners = useMemo(
    () =>
      vendors
        .filter((v) => v.tier)
        .sort(
          (a, b) =>
            PARTNER_TIER_RANK[a.tier ?? ''] - PARTNER_TIER_RANK[b.tier ?? ''] ||
            b.boards.length - a.boards.length ||
            byName(a, b)
        ),
    [vendors]
  );

  const recent = useMemo(() => {
    const ids: string[] = [];
    const others = (profiles ?? []).filter((p) => p.id !== profileId).sort((a, b) => b.updatedAt - a.updatedAt);
    for (const p of others) {
      for (const pb of p.boards ?? []) {
        const id = bySlug.get(pb.slug)?.vendor;
        if (id && !ids.includes(id)) ids.push(id);
      }
    }
    return ids
      .map((id) => vendorById.get(id))
      .filter((v): v is BoardVendor => !!v)
      .slice(0, UI.BOARD_SHEET.RECENT_BRANDS);
  }, [profiles, profileId, bySlug, vendorById]);

  return { boards, ordered, bySlug, vendors, vendorById, partners, recent, error, reload, showSkeleton };
}

/** Search, brand, paging, review and selection state of the Choose boards sheet; `selected` is owned by the caller. */
export function useBoardPicker(selected: ProfileBoard[], onChange: BoardsUpdate, profileId?: string) {
  const catalog = useBoardCatalog(profileId);
  const { ordered, bySlug, vendors, vendorById } = catalog;
  const { PER_PAGE, REVIEW_PER_PAGE } = UI.BOARD_SHEET;

  const [query, setQueryState] = useState('');
  const [vendor, setVendor] = useState('');
  const [page, setPageState] = useState(1);
  const [view, setViewState] = useState<'browse' | 'review'>('browse');
  const [reviewFilter, setReviewFilterState] = useState<ReviewFilter>(null);
  const [reviewPage, setReviewPageState] = useState(1);
  // +1 or -1 for a page move, 0 for a filter, null for typing: how PageSwap animates the next content
  const [motion, setMotion] = useState<number | null>(null);

  const picked = useMemo(() => new Set(selected.map((b) => b.slug)), [selected]);
  const needle = query.trim().toLowerCase();

  const shown = useMemo(() => {
    const pool = vendor ? ordered.filter((b) => b.vendor === vendor) : ordered;
    if (!needle) return pool;
    return pool
      .map((b, i) => ({ b, i, rank: bestMatchRank([b.name, b.vendor_name || '', b.slug], needle) }))
      .filter((x) => x.rank >= 0)
      .sort((x, y) => x.rank - y.rank || x.i - y.i)
      .map((x) => x.b);
  }, [ordered, vendor, needle]);
  const brandHits = useMemo(() => (needle ? rankVendors(vendors, needle) : []), [vendors, needle]);

  const pageCount = Math.max(1, Math.ceil(shown.length / PER_PAGE));
  const safePage = Math.min(page, pageCount);
  const pageItems = useMemo(() => shown.slice((safePage - 1) * PER_PAGE, safePage * PER_PAGE), [shown, safePage, PER_PAGE]);

  const picks = useMemo<PickedBoard[]>(
    () => selected.map((b) => ({ slug: b.slug, name: b.name, board: bySlug.get(b.slug) ?? null })),
    [selected, bySlug]
  );
  const reviewAll = useMemo(
    () =>
      [...picks].sort(
        (a, b) =>
          tierIndex(a.board?.support_tier) - tierIndex(b.board?.support_tier) ||
          (a.board?.vendor_name ?? '').localeCompare(b.board?.vendor_name ?? '') ||
          byName(a, b)
      ),
    [picks]
  );
  const activeFilter =
    reviewFilter &&
    picks.some((p) => (reviewFilter.kind === 'tier' ? p.board?.support_tier : p.board?.vendor) === reviewFilter.value)
      ? reviewFilter
      : null;
  const reviewShown = useMemo(
    () =>
      activeFilter
        ? reviewAll.filter((p) => (activeFilter.kind === 'tier' ? p.board?.support_tier : p.board?.vendor) === activeFilter.value)
        : reviewAll,
    [reviewAll, activeFilter]
  );
  const reviewPageCount = Math.max(1, Math.ceil(reviewShown.length / REVIEW_PER_PAGE));
  const safeReviewPage = Math.min(reviewPage, reviewPageCount);
  const reviewItems = useMemo(
    () => reviewShown.slice((safeReviewPage - 1) * REVIEW_PER_PAGE, safeReviewPage * REVIEW_PER_PAGE),
    [reviewShown, safeReviewPage, REVIEW_PER_PAGE]
  );

  const photos = useBoardPhotos(view === 'browse' ? pageItems.map((b) => b.slug) : reviewItems.map((p) => p.slug));

  const setQuery = (value: string) => {
    setQueryState(value);
    setPageState(1);
    setMotion(null);
  };
  const pickVendor = (id: string) => {
    setVendor(id);
    setQueryState('');
    setPageState(1);
    setMotion(0);
  };
  const setPage = (next: number) => {
    setMotion(Math.sign(next - safePage));
    setPageState(next);
  };
  const setReviewPage = (next: number) => {
    setMotion(Math.sign(next - safeReviewPage));
    setReviewPageState(next);
  };
  const setReviewFilter = (filter: ReviewFilter) => {
    setReviewFilterState(filter);
    setReviewPageState(1);
    setMotion(0);
  };
  const setView = (next: 'browse' | 'review') => {
    setViewState(next);
    setReviewFilterState(null);
    setReviewPageState(1);
    setMotion(null);
  };

  // Functional updates: clicks landing before a re-render must each see the previous click's result.
  const toggle = (board: BoardInfo) =>
    onChange((current) =>
      current.some((b) => b.slug === board.slug)
        ? current.filter((b) => b.slug !== board.slug)
        : [...current, toProfileBoard(board)]
    );
  const setMany = (list: BoardInfo[], on: boolean) =>
    onChange((current) => {
      const slugs = new Set(list.map((b) => b.slug));
      const rest = current.filter((b) => !slugs.has(b.slug));
      return on ? [...rest, ...list.map(toProfileBoard)] : rest;
    });
  const remove = (slug: string) => {
    setMotion(0);
    onChange((current) => current.filter((b) => b.slug !== slug));
  };
  const replace = (boards: ProfileBoard[]) => {
    setMotion(0);
    onChange(() => boards);
  };

  return {
    ...catalog,
    query,
    setQuery,
    needle,
    vendor: vendorById.get(vendor) ?? null,
    pickVendor,
    shown,
    brandHits,
    page: safePage,
    pageCount,
    setPage,
    pageItems,
    picked,
    picks,
    view,
    setView,
    reviewAll,
    reviewFilter: activeFilter,
    setReviewFilter,
    reviewShown,
    reviewItems,
    reviewPage: safeReviewPage,
    reviewPageCount,
    setReviewPage,
    motion,
    photos,
    toggle,
    setMany,
    remove,
    replace,
    vendorCount: vendors.length,
  };
}

export type BoardPickerState = ReturnType<typeof useBoardPicker>;
