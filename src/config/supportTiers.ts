/** Support tier identifiers, labels, and ordering shared across panels and modals */

/** Canonical support tier identifiers */
export const SUPPORT_TIER = {
  PLATINUM: 'platinum',
  STANDARD: 'standard',
  COMMUNITY: 'community',
  EOS: 'eos',
  TVB: 'tvb',
  WIP: 'wip',
} as const;

/** Human-readable label for each support tier */
export const SUPPORT_TIER_LABEL: Record<string, string> = {
  platinum: 'Platinum',
  standard: 'Standard',
  community: 'Community',
  eos: 'EOS',
  tvb: 'TV Box',
  wip: 'WIP',
};

/** Support tier priority order (lower index = higher priority) */
export const SUPPORT_TIER_ORDER: string[] = ['platinum', 'standard', 'community', 'eos', 'tvb', 'wip'];

/** Partner tier ranking used to sort manufacturers (lower rank = higher priority) */
export const PARTNER_TIER_RANK: Record<string, number> = { platinum: 0, gold: 1, silver: 2 };
