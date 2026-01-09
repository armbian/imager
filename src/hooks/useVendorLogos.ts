import { useState, useEffect, useMemo, useCallback } from 'react';
import type { BoardInfo } from '../types';
import { preloadImage } from '../utils';
import { VENDOR } from '../config';

interface VendorLogoState {
  failedLogos: Set<string>;
  isLoaded: boolean;
}

/**
 * Hook to validate vendor logos by preloading them and tracking failures.
 * Vendors with failed logos (404, network errors, etc.) are grouped under "other".
 */
export function useVendorLogos(boards: BoardInfo[] | null, isActive: boolean) {
  const [state, setState] = useState<VendorLogoState>({
    failedLogos: new Set(),
    isLoaded: false,
  });

  // Reset state when inactive
  useEffect(() => {
    if (!isActive) {
      setState({ failedLogos: new Set(), isLoaded: false });
    }
  }, [isActive]);

  // Preload logos and track failures
  useEffect(() => {
    if (!isActive || !boards?.length || state.isLoaded) return;

    const vendorLogos = new Map<string, string>();
    for (const board of boards) {
      if (board.vendor && board.vendor !== VENDOR.FALLBACK_ID && board.vendor_logo) {
        vendorLogos.set(board.vendor, board.vendor_logo);
      }
    }

    if (vendorLogos.size === 0) {
      setState({ failedLogos: new Set(), isLoaded: true });
      return;
    }

    let loaded = 0;
    const failed = new Set<string>();

    vendorLogos.forEach((logoUrl, vendorId) => {
      preloadImage(logoUrl).then((success) => {
        if (!success) {
          failed.add(vendorId);
        }
        loaded++;
        if (loaded >= vendorLogos.size) {
          setState({ failedLogos: failed, isLoaded: true });
        }
      });
    });
  }, [isActive, boards, state.isLoaded]);

  // Helper to get effective vendor (considering failed logos)
  const getEffectiveVendor = useCallback((board: BoardInfo): string => {
    if (!board.vendor_logo || state.failedLogos.has(board.vendor)) {
      return VENDOR.FALLBACK_ID;
    }
    return board.vendor || VENDOR.FALLBACK_ID;
  }, [state.failedLogos]);

  // Check if a vendor has a valid logo
  const hasValidLogo = useCallback((board: BoardInfo): boolean => {
    return !!(board.vendor_logo && !state.failedLogos.has(board.vendor));
  }, [state.failedLogos]);

  return {
    failedLogos: state.failedLogos,
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

/**
 * Hook to build manufacturer list from boards with validated logos.
 * Boards with failed logos are grouped under "other".
 */
export function useManufacturerList(
  boards: BoardInfo[] | null,
  isActive: boolean,
  searchFilter: string = ''
) {
  const { failedLogos, isLoaded, getEffectiveVendor, hasValidLogo } = useVendorLogos(boards, isActive);

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

    // Build vendor map with board counts, platinum board counts, and standard board counts
    for (const board of boards) {
      const validLogo = hasValidLogo(board);
      const vendorId = validLogo ? (board.vendor || VENDOR.FALLBACK_ID) : VENDOR.FALLBACK_ID;
      const vendorName = validLogo ? (board.vendor_name || 'Other') : 'Other';
      const vendorLogo = validLogo ? board.vendor_logo : null;

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

      // Increment platinum count if this board has platinum support
      if (board.has_platinum_support) {
        vendorMap[vendorId].platinumCount++;
      }

      // Increment standard count if this board has standard support
      if (board.has_standard_support) {
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
        // "Other" category always goes to the bottom
        if (a.id === VENDOR.FALLBACK_ID) return 1;
        if (b.id === VENDOR.FALLBACK_ID) return -1;

        // Tier 1: Vendors with MORE than 1 platinum board (highest priority)
        const aMultiPlatinum = a.platinumCount > 1;
        const bMultiPlatinum = b.platinumCount > 1;

        if (aMultiPlatinum && !bMultiPlatinum) return -1;
        if (!aMultiPlatinum && bMultiPlatinum) return 1;

        // Within Tier 1, sort by platinum count (descending)
        if (aMultiPlatinum && bMultiPlatinum) {
          if (a.platinumCount !== b.platinumCount) {
            return b.platinumCount - a.platinumCount;
          }
          return b.boardCount - a.boardCount;
        }

        // Tier 2: Vendors with exactly 1 platinum board (any platinum beats standard-only)
        const aSinglePlatinum = a.platinumCount === 1;
        const bSinglePlatinum = b.platinumCount === 1;

        if (aSinglePlatinum && !bSinglePlatinum) return -1;
        if (!aSinglePlatinum && bSinglePlatinum) return 1;

        // Within Tier 2, sort by standard count then board count
        if (aSinglePlatinum && bSinglePlatinum) {
          if (a.standardCount !== b.standardCount) {
            return b.standardCount - a.standardCount;
          }
          return b.boardCount - a.boardCount;
        }

        // Tier 3: Vendors with MORE than 1 standard board (no platinum)
        const aMultiStandard = a.standardCount > 1;
        const bMultiStandard = b.standardCount > 1;

        if (aMultiStandard && !bMultiStandard) return -1;
        if (!aMultiStandard && bMultiStandard) return 1;

        // Within Tier 3, sort by standard count (descending)
        if (aMultiStandard && bMultiStandard) {
          if (a.standardCount !== b.standardCount) {
            return b.standardCount - a.standardCount;
          }
          return b.boardCount - a.boardCount;
        }

        // Tier 4: Remaining vendors - sort by total board count (descending)
        return b.boardCount - a.boardCount;
      });

    return result;
  }, [boards, isLoaded, searchFilter, hasValidLogo]);

  return {
    manufacturers,
    isLoaded,
    failedLogos,
    getEffectiveVendor,
  };
}
