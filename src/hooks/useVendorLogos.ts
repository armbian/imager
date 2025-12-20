import { useState, useEffect, useMemo, useCallback } from 'react';
import type { BoardInfo } from '../types';
import { preloadImage } from '../utils';

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
      if (board.vendor && board.vendor !== 'other' && board.vendor_logo) {
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
      return 'other';
    }
    return board.vendor || 'other';
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
    const vendorMap: Record<string, { name: string; logo: string | null; count: number }> = {};

    for (const board of boards) {
      const validLogo = hasValidLogo(board);
      const vendorId = validLogo ? (board.vendor || 'other') : 'other';
      const vendorName = validLogo ? (board.vendor_name || 'Other') : 'Other';
      const vendorLogo = validLogo ? board.vendor_logo : null;

      if (!vendorMap[vendorId]) {
        vendorMap[vendorId] = {
          name: vendorName,
          logo: vendorLogo,
          count: 0,
        };
      }
      vendorMap[vendorId].count++;
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
      }))
      .sort((a, b) => {
        if (a.id === 'other') return 1;
        if (b.id === 'other') return -1;
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
