/**
 * Skeleton loading components for cards and lists
 * Displays animated placeholders while content is loading
 */

import { UI } from '../../config';

interface BoardCardSkeletonProps {
  count?: number;
}

/**
 * Skeleton component for board grid cards
 * Shows placeholders matching the board card layout
 */
export function BoardCardSkeleton({ count = UI.SKELETON.BOARD_GRID_COUNT }: BoardCardSkeletonProps) {
  return (
    <>
      {Array.from({ length: count }).map((_, index) => (
        <div key={`skeleton-${index}`} className="board-card-skeleton">
          <div className="board-card-skeleton-image skeleton" />
          <div className="board-card-skeleton-info">
            <div className="board-card-skeleton-name skeleton" />
            <div className="board-card-skeleton-badges">
              <div className="board-card-skeleton-badge skeleton" />
              <div className="board-card-skeleton-badge skeleton" />
            </div>
          </div>
        </div>
      ))}
    </>
  );
}

interface ListItemSkeletonProps {
  count?: number;
}

/**
 * Skeleton component for list items
 * Shows placeholders matching the list item layout
 */
export function ListItemSkeleton({ count = UI.SKELETON.LIST_COUNT }: ListItemSkeletonProps) {
  return (
    <>
      {Array.from({ length: count }).map((_, index) => (
        <div key={`skeleton-${index}`} className="list-item-skeleton">
          <div className="list-item-skeleton-icon skeleton" />
          <div className="list-item-skeleton-content">
            <div className="list-item-skeleton-title skeleton" />
            <div className="list-item-skeleton-subtitle skeleton" />
          </div>
        </div>
      ))}
    </>
  );
}
