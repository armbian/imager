/**
 * Board support badges component
 *
 * Displays support status badges for boards (Platinum, Standard, Community, EOS, TV Box, WIP)
 * Consolidated from BoardModal and ArmbianBoardModal to eliminate duplication
 */

import { Crown, Shield, Users, Clock, Tv, Wrench } from 'lucide-react';
import type { BoardInfo } from '../../types';

interface BoardBadgesProps {
  board: BoardInfo;
  className?: string;
}

export function BoardBadges({ board, className = '' }: BoardBadgesProps) {
  return (
    <div className={`board-grid-badges ${className}`}>
      {board.support_tier === 'platinum' && (
        <span className="badge-platinum">
          <Crown size={10} />
          <span>Platinum</span>
        </span>
      )}
      {board.support_tier === 'standard' && (
        <span className="badge-standard">
          <Shield size={10} />
          <span>Standard</span>
        </span>
      )}
      {board.support_tier === 'community' && (
        <span className="badge-community">
          <Users size={10} />
          <span>Community</span>
        </span>
      )}
      {board.support_tier === 'eos' && (
        <span className="badge-eos">
          <Clock size={10} />
          <span>EOS</span>
        </span>
      )}
      {board.support_tier === 'tvb' && (
        <span className="badge-tvb">
          <Tv size={10} />
          <span>TV Box</span>
        </span>
      )}
      {board.support_tier === 'wip' && (
        <span className="badge-wip">
          <Wrench size={10} />
          <span>WIP</span>
        </span>
      )}
    </div>
  );
}
