import { useState, useEffect, useCallback, useRef } from 'react';
import { getErrorMessage } from '../utils';

/** Result of async data fetching */
export interface AsyncDataResult<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  reload: () => Promise<void>;
}

/** Options for useAsyncData hook */
export interface UseAsyncDataOptions {
  /** Fetch immediately on mount (default: true) */
  immediate?: boolean;
  /** Reset data to null before reloading (default: false) */
  resetOnReload?: boolean;
}

/** Core async fetching logic shared by useAsyncData and useAsyncDataWhen */
function useAsyncDataCore<T>(
  fetcher: () => Promise<T>,
  options: { resetOnReload?: boolean; initialLoading?: boolean } = {}
): AsyncDataResult<T> & { triggerFetch: () => void } {
  const { resetOnReload = false, initialLoading = false } = options;

  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(initialLoading);
  const [error, setError] = useState<string | null>(null);

  // Prevent state updates after unmount
  const mountedRef = useRef(true);
  // Discards stale results when a newer fetch supersedes this one
  const fetchIdRef = useRef(0);
  // Avoids recreating the reload callback when fetcher changes
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  const reload = useCallback(async () => {
    const currentFetchId = ++fetchIdRef.current;

    if (resetOnReload) {
      setData(null);
    }
    setLoading(true);
    setError(null);

    try {
      const result = await fetcherRef.current();

      // Skip if a newer fetch superseded this one or the component unmounted
      if (mountedRef.current && currentFetchId === fetchIdRef.current) {
        setData(result);
        setError(null);
      }
    } catch (err) {
      if (mountedRef.current && currentFetchId === fetchIdRef.current) {
        setError(getErrorMessage(err));
      }
    } finally {
      if (mountedRef.current && currentFetchId === fetchIdRef.current) {
        setLoading(false);
      }
    }
  }, [resetOnReload]);

  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return { data, loading, error, reload, triggerFetch: reload };
}

/** Async data fetching with loading/error state, refetching when `deps` change */
export function useAsyncData<T>(
  fetcher: () => Promise<T>,
  deps: React.DependencyList = [],
  options: UseAsyncDataOptions = {}
): AsyncDataResult<T> {
  const { immediate = true, resetOnReload = false } = options;

  const { data, loading, error, reload } = useAsyncDataCore(fetcher, {
    resetOnReload,
    initialLoading: immediate
  });

  useEffect(() => {
    if (immediate) {
      reload();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, immediate]);

  return { data, loading, error, reload };
}
