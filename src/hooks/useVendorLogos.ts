import { useState, useEffect, useMemo, useCallback } from 'react';
import type { BoardInfo } from '../types';
import { getCachedVendorLogo } from './useTauri';
import { VENDOR } from '../config';

interface VendorLogoState {
  failedLogos: Set<string>;
  /** Maps vendor ID to cached data URI (base64-encoded) */
  cachedUrls: Map<string, string>;
  isLoaded: boolean;
}

/** Preload vendor logos and track failures; failed vendors are grouped under "other" */
export function useVendorLogos(boards: BoardInfo[] | null, isActive: boolean) {
  const [state, setState] = useState<VendorLogoState>({
    failedLogos: new Set(),
    cachedUrls: new Map(),
    isLoaded: false,
  });

  // Reset state when inactive
  useEffect(() => {
    if (!isActive) {
      setState({ failedLogos: new Set(), cachedUrls: new Map(), isLoaded: false });
    }
  }, [isActive]);

  // Preload logos via local cache using vendor slugs
  useEffect(() => {
    if (!isActive || !boards?.length || state.isLoaded) return;

    const vendorSlugs = new Set<string>();
    for (const board of boards) {
      if (board.vendor && board.vendor !== VENDOR.FALLBACK_ID) {
        vendorSlugs.add(board.vendor);
      }
    }

    if (vendorSlugs.size === 0) {
      setState({ failedLogos: new Set(), cachedUrls: new Map(), isLoaded: true });
      return;
    }

    let loaded = 0;
    const failed = new Set<string>();
    const cached = new Map<string, string>();

    vendorSlugs.forEach((vendorSlug) => {
      getCachedVendorLogo(vendorSlug).then((dataUri) => {
        if (dataUri) {
          cached.set(vendorSlug, dataUri);
        } else {
          failed.add(vendorSlug);
        }
      }).catch(() => {
        failed.add(vendorSlug);
      }).finally(() => {
        loaded++;
        if (loaded >= vendorSlugs.size) {
          setState({ failedLogos: failed, cachedUrls: cached, isLoaded: true });
        }
      });
    });
  }, [isActive, boards, state.isLoaded]);

  // Falls back to "other" when the vendor's logo failed to load
  const getEffectiveVendor = useCallback((board: BoardInfo): string => {
    if (!board.vendor || state.failedLogos.has(board.vendor)) {
      return VENDOR.FALLBACK_ID;
    }
    return board.vendor || VENDOR.FALLBACK_ID;
  }, [state.failedLogos]);

  const hasValidLogo = useCallback((board: BoardInfo): boolean => {
    return !!(board.vendor && !state.failedLogos.has(board.vendor));
  }, [state.failedLogos]);

  return {
    failedLogos: state.failedLogos,
    cachedUrls: state.cachedUrls,
    isLoaded: state.isLoaded,
    getEffectiveVendor,
    hasValidLogo,
  };
}

export interface ManufacturerData {
  id: string;
  name: string;
  logo: string | null;
  boardCount: number;
  platinumCount: number;
  standardCount: number;
}

/** Build the manufacturer list from boards; boards with failed logos go under "other" */
export function useManufacturerList(
  boards: BoardInfo[] | null,
  isActive: boolean,
  searchFilter: string = ''
) {
  const { failedLogos, cachedUrls, isLoaded, getEffectiveVendor, hasValidLogo } = useVendorLogos(boards, isActive);

  const manufacturers = useMemo(() => {
    if (!boards || !isLoaded) return [];

    const searchLower = searchFilter.toLowerCase();
    const vendorMap: Record<string, {
      name: string;
      logo: string | null;
      count: number;
      platinumCount: number;
      standardCount: number;
    }> = {};

    for (const board of boards) {
      const validLogo = hasValidLogo(board);
      const vendorId = validLogo ? (board.vendor || VENDOR.FALLBACK_ID) : VENDOR.FALLBACK_ID;
      const vendorName = validLogo ? (board.vendor_name || 'Other') : 'Other';
      const vendorLogo = validLogo
        ? (cachedUrls.get(board.vendor) || null)
        : null;

      if (!vendorMap[vendorId]) {
        vendorMap[vendorId] = {
          name: vendorName,
          logo: vendorLogo,
          count: 0,
          platinumCount: 0,
          standardCount: 0,
        };
      }
      vendorMap[vendorId].count++;

      if (board.support_tier === 'platinum') {
        vendorMap[vendorId].platinumCount++;
      }
      if (board.support_tier === 'standard') {
        vendorMap[vendorId].standardCount++;
      }
    }

    const result: ManufacturerData[] = Object.entries(vendorMap)
      .filter(([, data]) => {
        if (data.count === 0) return false;
        return data.name.toLowerCase().includes(searchLower);
      })
      .map(([id, data]) => ({
        id,
        name: data.name,
        logo: data.logo,
        boardCount: data.count,
        platinumCount: data.platinumCount,
        standardCount: data.standardCount,
      }))
      .sort((a, b) => {
        // "Other" always last
        if (a.id === VENDOR.FALLBACK_ID) return 1;
        if (b.id === VENDOR.FALLBACK_ID) return -1;

        // Tier 1: >1 platinum board
        const aMultiPlatinum = a.platinumCount > 1;
        const bMultiPlatinum = b.platinumCount > 1;

        if (aMultiPlatinum && !bMultiPlatinum) return -1;
        if (!aMultiPlatinum && bMultiPlatinum) return 1;

        if (aMultiPlatinum && bMultiPlatinum) {
          if (a.platinumCount !== b.platinumCount) {
            return b.platinumCount - a.platinumCount;
          }
          return b.boardCount - a.boardCount;
        }

        // Tier 2: exactly 1 platinum board (beats standard-only)
        const aSinglePlatinum = a.platinumCount === 1;
        const bSinglePlatinum = b.platinumCount === 1;

        if (aSinglePlatinum && !bSinglePlatinum) return -1;
        if (!aSinglePlatinum && bSinglePlatinum) return 1;

        if (aSinglePlatinum && bSinglePlatinum) {
          if (a.standardCount !== b.standardCount) {
            return b.standardCount - a.standardCount;
          }
          return b.boardCount - a.boardCount;
        }

        // Tier 3: >1 standard board, no platinum
        const aMultiStandard = a.standardCount > 1;
        const bMultiStandard = b.standardCount > 1;

        if (aMultiStandard && !bMultiStandard) return -1;
        if (!aMultiStandard && bMultiStandard) return 1;

        if (aMultiStandard && bMultiStandard) {
          if (a.standardCount !== b.standardCount) {
            return b.standardCount - a.standardCount;
          }
          return b.boardCount - a.boardCount;
        }

        // Tier 4: remaining vendors by total board count
        return b.boardCount - a.boardCount;
      });

    return result;
  }, [boards, isLoaded, searchFilter, hasValidLogo, cachedUrls]);

  return {
    manufacturers,
    isLoaded,
    failedLogos,
    getEffectiveVendor,
  };
}
