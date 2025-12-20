/**
 * Reusable loading state component for modal dialogs
 */

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

interface LoadingStateProps {
  /** Whether content is loading */
  isLoading: boolean;
  /** Loading message translation key (defaults to 'modal.loading') */
  messageKey?: string;
  /** Children to render when not loading */
  children: ReactNode;
}

/**
 * Displays a loading spinner when isLoading is true, otherwise renders children
 */
export function LoadingState({
  isLoading,
  messageKey = 'modal.loading',
  children
}: LoadingStateProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div className="loading">
        <div className="spinner" />
        <p>{t(messageKey)}</p>
      </div>
    );
  }

  return <>{children}</>;
}
