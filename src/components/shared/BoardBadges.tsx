// Support tier badges (Platinum, Standard, Community, EOS, TV Box, WIP)

import { Crown, Shield, Users, Clock, Tv, Wrench } from 'lucide-react';
import type { BoardInfo } from '../../types';
import { SUPPORT_TIER, SUPPORT_TIER_LABEL } from '../../config';

interface BoardBadgesProps {
  board: BoardInfo;
  className?: string;
}

export function BoardBadges({ board, className = '' }: BoardBadgesProps) {
  return (
    <div className={`board-grid-badges ${className}`}>
      {board.support_tier === SUPPORT_TIER.PLATINUM && (
        <span className="badge-platinum">
          <Crown size={10} />
          <span>{SUPPORT_TIER_LABEL[SUPPORT_TIER.PLATINUM]}</span>
        </span>
      )}
      {board.support_tier === SUPPORT_TIER.STANDARD && (
        <span className="badge-standard">
          <Shield size={10} />
          <span>{SUPPORT_TIER_LABEL[SUPPORT_TIER.STANDARD]}</span>
        </span>
      )}
      {board.support_tier === SUPPORT_TIER.COMMUNITY && (
        <span className="badge-community">
          <Users size={10} />
          <span>{SUPPORT_TIER_LABEL[SUPPORT_TIER.COMMUNITY]}</span>
        </span>
      )}
      {board.support_tier === SUPPORT_TIER.EOS && (
        <span className="badge-eos">
          <Clock size={10} />
          <span>{SUPPORT_TIER_LABEL[SUPPORT_TIER.EOS]}</span>
        </span>
      )}
      {board.support_tier === SUPPORT_TIER.TVB && (
        <span className="badge-tvb">
          <Tv size={10} />
          <span>{SUPPORT_TIER_LABEL[SUPPORT_TIER.TVB]}</span>
        </span>
      )}
      {board.support_tier === SUPPORT_TIER.WIP && (
        <span className="badge-wip">
          <Wrench size={10} />
          <span>{SUPPORT_TIER_LABEL[SUPPORT_TIER.WIP]}</span>
        </span>
      )}
    </div>
  );
}
