import { useState, useEffect } from 'react';

/** Skeleton loading with a min visibility window to avoid flicker: shows on load, hides
 * only after data is ready and `minDuration` (default 300ms) elapses. */
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
