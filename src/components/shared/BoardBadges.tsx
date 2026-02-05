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
      {board.has_platinum_support && (
        <span className="badge-platinum">
          <Crown size={10} />
          <span>Platinum</span>
        </span>
      )}
      {board.has_standard_support && !board.has_platinum_support && (
        <span className="badge-standard">
          <Shield size={10} />
          <span>Standard</span>
        </span>
      )}
      {board.has_community_support && (
        <span className="badge-community">
          <Users size={10} />
          <span>Community</span>
        </span>
      )}
      {board.has_eos_support && (
        <span className="badge-eos">
          <Clock size={10} />
          <span>EOS</span>
        </span>
      )}
      {board.has_tvb_support && (
        <span className="badge-tvb">
          <Tv size={10} />
          <span>TV Box</span>
        </span>
      )}
      {board.has_wip_support && (
        <span className="badge-wip">
          <Wrench size={10} />
          <span>WIP</span>
        </span>
      )}
    </div>
  );
}
