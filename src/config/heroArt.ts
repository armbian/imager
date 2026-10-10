// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { HeroId, SettingsView } from '../types';
import generalMoon from '../assets/settings/general/general-moon.webp';
import generalSun from '../assets/settings/general/general-sun.webp';
import generalIslandDay from '../assets/settings/general/general-island-day.webp';
import generalIslandNight from '../assets/settings/general/general-island-night.webp';
import generalWindow from '../assets/settings/general/general-window.webp';
import generalCloud1 from '../assets/settings/general/general-cloud-1.webp';
import generalCloud2 from '../assets/settings/general/general-cloud-2.webp';
import generalStar1 from '../assets/settings/general/general-star-1.webp';
import generalStar2 from '../assets/settings/general/general-star-2.webp';
import generalStar3 from '../assets/settings/general/general-star-3.webp';
import generalStar4 from '../assets/settings/general/general-star-4.webp';
import generalStar5 from '../assets/settings/general/general-star-5.webp';
import writingPlate from '../assets/settings/writing/writing-plate.webp';
import writingLight from '../assets/settings/writing/writing-light.webp';
import writingCard from '../assets/settings/writing/writing-card.webp';
import writingPillar from '../assets/settings/writing/writing-pillar.webp';
import writingLightEdge from '../assets/settings/writing/writing-light-edge.webp';
import writingBar from '../assets/settings/writing/writing-bar.webp';
import writingDome from '../assets/settings/writing/writing-dome.webp';
import writingCheck from '../assets/settings/writing/writing-check.webp';
import downloadsTray from '../assets/settings/downloads/downloads-tray.webp';
import downloadsTrayDark from '../assets/settings/downloads/downloads-tray-dark.webp';
import downloadsGlow from '../assets/settings/downloads/downloads-glow.webp';
import downloadsArrow from '../assets/settings/downloads/downloads-arrow.webp';
import downloadsArrowGrey from '../assets/settings/downloads/downloads-arrow-grey.webp';
import downloadsDot from '../assets/settings/downloads/downloads-dot.webp';
import downloadsDotGrey from '../assets/settings/downloads/downloads-dot-grey.webp';
import downloadsDotGreyDark from '../assets/settings/downloads/downloads-dot-grey-dark.webp';
import downloadsArc1 from '../assets/settings/downloads/downloads-arc-1.webp';
import downloadsArc2 from '../assets/settings/downloads/downloads-arc-2.webp';
import downloadsArc3 from '../assets/settings/downloads/downloads-arc-3.webp';
import downloadsArc1Grey from '../assets/settings/downloads/downloads-arc-1-grey.webp';
import downloadsArc1GreyDark from '../assets/settings/downloads/downloads-arc-1-grey-dark.webp';
import downloadsArc2Grey from '../assets/settings/downloads/downloads-arc-2-grey.webp';
import downloadsArc3Grey from '../assets/settings/downloads/downloads-arc-3-grey.webp';
import downloadsArc3GreyDark from '../assets/settings/downloads/downloads-arc-3-grey-dark.webp';
import downloadsSlash from '../assets/settings/downloads/downloads-slash.webp';
import downloadsSlashDark from '../assets/settings/downloads/downloads-slash-dark.webp';
import profilesWifi from '../assets/settings/profiles/profiles-wifi.webp';
import profilesKey from '../assets/settings/profiles/profiles-key.webp';
import profilesGlobe from '../assets/settings/profiles/profiles-globe.webp';
import profilesLock from '../assets/settings/profiles/profiles-lock.webp';
import profilesCard from '../assets/settings/profiles/profiles-card.webp';
import profilesEmptyCard from '../assets/settings/profiles/profiles-empty-card.webp';
import profilesEmptyWifi from '../assets/settings/profiles/profiles-empty-wifi.webp';
import profilesEmptyPerson from '../assets/settings/profiles/profiles-empty-person.webp';
import profilesEmptyKey from '../assets/settings/profiles/profiles-empty-key.webp';
import profilesEmptyPlus from '../assets/settings/profiles/profiles-empty-plus.webp';
import navGeneral from '../assets/settings/nav/nav-general.png';
import navWriting from '../assets/settings/nav/nav-writing.png';
import navProfiles from '../assets/settings/nav/nav-profiles.png';
import navStorage from '../assets/settings/nav/nav-storage.png';
import navAbout from '../assets/settings/nav/nav-about.png';
import allBoards from '../assets/settings/profiles/all-boards.png';

interface HeroLayer {
  id: string;
  src: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Used instead of src under the dark theme, for parts whose rim is lit for the light card */
  srcDark?: string;
  origin?: string;
}

interface HeroArtSpec {
  canvas: { w: number; h: number };
  layers: readonly HeroLayer[];
  /** When the entrance hands over to the idle loops; HERO_SETTLE_MS when absent */
  settleMs?: number;
}

// Source px of the prototype art; the canvas is the visible art box, so soft edges may overhang it.
export const HERO_ART: Record<HeroId, HeroArtSpec> = {
  general: {
    canvas: { w: 379, h: 352 },
    settleMs: 1900,
    layers: [
      { id: 'moon', src: generalMoon, x: 237, y: 0, w: 131, h: 125 },
      { id: 'sun', src: generalSun, x: 12, y: 21, w: 82, h: 81 },
      { id: 'day', src: generalIslandDay, x: 1, y: 101, w: 346, h: 249 },
      { id: 'night', src: generalIslandNight, x: 1, y: 101, w: 346, h: 249 },
      { id: 'window', src: generalWindow, x: 141, y: 144, w: 83, h: 84 },
      { id: 'cloud1', src: generalCloud1, x: 218, y: 30, w: 106, h: 70 },
      { id: 'cloud2', src: generalCloud2, x: 296, y: 89, w: 77, h: 52 },
      { id: 'star1', src: generalStar1, x: 29, y: 25, w: 61, h: 60 },
      { id: 'star2', src: generalStar2, x: 85, y: 71, w: 40, h: 39 },
      { id: 'star3', src: generalStar3, x: 155, y: 17, w: 50, h: 50 },
      { id: 'star4', src: generalStar4, x: 210, y: 79, w: 36, h: 36 },
      { id: 'star5', src: generalStar5, x: 334, y: 94, w: 45, h: 45 },
    ],
  },
  writing: {
    canvas: { w: 509, h: 352 },
    settleMs: 1900,
    layers: [
      { id: 'plate', src: writingPlate, x: 2, y: 2, w: 505, h: 349 },
      { id: 'light', src: writingLight, x: 205, y: 52, w: 147, h: 183 },
      { id: 'card1', src: writingCard, x: 85, y: 184, w: 143, h: 83, origin: '48% 46.67%' },
      { id: 'card2', src: writingCard, x: 85, y: 184, w: 143, h: 83, origin: '48% 46.67%' },
      { id: 'card3', src: writingCard, x: 85, y: 184, w: 143, h: 83, origin: '48% 46.67%' },
      { id: 'pillar', src: writingPillar, x: 339, y: 84, w: 91, h: 210 },
      { id: 'lightEdge', src: writingLightEdge, x: 339, y: 140, w: 14, h: 73 },
      { id: 'bar', src: writingBar, x: 212, y: 4, w: 124, h: 138 },
      { id: 'dome', src: writingDome, x: 242, y: 2, w: 96, h: 90, origin: '50.5% 94.22%' },
      { id: 'check', src: writingCheck, x: 251, y: 23, w: 60, h: 57, origin: '65.8% 42.67%' },
    ],
  },
  downloads: {
    canvas: { w: 352, h: 352 },
    settleMs: 1800,
    layers: [
      { id: 'tray', src: downloadsTray, srcDark: downloadsTrayDark, x: 43, y: 152, w: 278, h: 182 },
      { id: 'glow', src: downloadsGlow, x: 45, y: 154, w: 273, h: 149 },
      { id: 'arrow', src: downloadsArrow, x: 143, y: 51, w: 118, h: 120, origin: '49.69% 95.98%' },
      { id: 'arrowGrey', src: downloadsArrowGrey, x: 143, y: 51, w: 118, h: 120, origin: '49.69% 95.98%' },
      { id: 'dot', src: downloadsDot, x: 90, y: 98, w: 36, h: 36, origin: '49.49% 48.18%' },
      { id: 'arc1', src: downloadsArc1, x: 72, y: 75, w: 69, h: 37, origin: '51.91% 109.04%' },
      { id: 'arc2', src: downloadsArc2, x: 56, y: 51, w: 97, h: 48, origin: '53.42% 134.05%' },
      { id: 'arc3', src: downloadsArc3, x: 40, y: 27, w: 124, h: 57, origin: '54.69% 154.99%' },
      { id: 'dotGrey', src: downloadsDotGrey, srcDark: downloadsDotGreyDark, x: 90, y: 98, w: 36, h: 36, origin: '49.49% 48.18%' },
      { id: 'arc1Grey', src: downloadsArc1Grey, srcDark: downloadsArc1GreyDark, x: 72, y: 75, w: 69, h: 37, origin: '51.91% 109.04%' },
      { id: 'arc2Grey', src: downloadsArc2Grey, x: 56, y: 51, w: 97, h: 48, origin: '53.42% 134.05%' },
      { id: 'arc3Grey', src: downloadsArc3Grey, srcDark: downloadsArc3GreyDark, x: 40, y: 27, w: 124, h: 57, origin: '54.69% 154.99%' },
      { id: 'slash', src: downloadsSlash, srcDark: downloadsSlashDark, x: 46, y: 25, w: 106, h: 109, origin: '49.82% 49.94%' },
    ],
  },
  profiles: {
    canvas: { w: 508, h: 352 },
    settleMs: 2200,
    layers: [
      { id: 'wifi', src: profilesWifi, x: 16, y: 2, w: 135, h: 145 },
      { id: 'key', src: profilesKey, x: 356, y: 12, w: 146, h: 151 },
      { id: 'globe', src: profilesGlobe, x: 363, y: 194, w: 143, h: 156 },
      { id: 'lock', src: profilesLock, x: 2, y: 182, w: 145, h: 157 },
      { id: 'card', src: profilesCard, x: 163, y: 50, w: 183, h: 254, origin: '50.39% 98.12%' },
    ],
  },
  profilesEmpty: {
    canvas: { w: 427, h: 352 },
    settleMs: 1900,
    layers: [
      { id: 'card', src: profilesEmptyCard, x: 88, y: 12, w: 246, h: 279, origin: '42.01% 108.83%' },
      { id: 'wifi', src: profilesEmptyWifi, x: 2, y: 2, w: 153, h: 147, origin: '52.08% 96.62%' },
      { id: 'person', src: profilesEmptyPerson, x: 315, y: 76, w: 110, h: 137, origin: '52.54% 96.33%' },
      { id: 'key', src: profilesEmptyKey, x: 243, y: 214, w: 169, h: 136, origin: '54.85% 96.88%' },
      { id: 'plus', src: profilesEmptyPlus, x: 17, y: 192, w: 134, h: 128, origin: '50.37% 96.58%' },
    ],
  },
};

export const NAV_ICONS: Record<SettingsView, string> = {
  general: navGeneral,
  writing: navWriting,
  profiles: navProfiles,
  downloads: navStorage,
  about: navAbout,
};

export const ALL_BOARDS_ART = allBoards;

// Hand-over from entrance to idle loops for a hero without its own settleMs.
export const HERO_SETTLE_MS = 1600;
