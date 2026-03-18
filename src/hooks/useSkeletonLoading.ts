import { useState, useEffect } from 'react';

/**
 * Shared hook for skeleton loading with minimum visibility delay
 *
 * Shows skeleton immediately when loading starts, then hides it
 * after data is ready with a minimum display duration to prevent flickering.
 *
 * @param loading - Whether data is currently being fetched
 * @param isReady - Whether the data is ready to display (may differ from !loading)
 * @param minDuration - Minimum skeleton display time in ms (default 300)
 * @returns Object with showSkeleton boolean
 */
export function useSkeletonLoading(
  loading: boolean,
  isReady: boolean,
  minDuration: number = 300
): { showSkeleton: boolean } {
  const [showSkeleton, setShowSkeleton] = useState(false);

  useEffect(() => {
    let skeletonTimeout: NodeJS.Timeout;

    if (loading) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Show skeleton during loading
      setShowSkeleton(true);
    } else if (isReady) {
      // Keep skeleton visible for at least minDuration ms
      skeletonTimeout = setTimeout(() => {
        setShowSkeleton(false);
      }, minDuration);
    }

    return () => {
      if (skeletonTimeout) {
        clearTimeout(skeletonTimeout);
      }
    };
  }, [loading, isReady, minDuration]);

  return { showSkeleton };
}
