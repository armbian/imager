// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { HeroId, SettingsView } from '../types';
import generalBaseNoknob from '../assets/settings/general/general-base-noknob.png';
import generalTrackGrey from '../assets/settings/general/general-track-grey.png';
import generalDiscDim from '../assets/settings/general/general-disc-dim.png';
import generalKnob from '../assets/settings/general/general-knob.png';
import writingBase from '../assets/settings/writing/writing-base.png';
import writingShieldGrey from '../assets/settings/writing/writing-shield-grey.png';
import writingArc01 from '../assets/settings/writing/writing-arc-01.png';
import writingArc02 from '../assets/settings/writing/writing-arc-02.png';
import writingArc03 from '../assets/settings/writing/writing-arc-03.png';
import writingArc04 from '../assets/settings/writing/writing-arc-04.png';
import writingArc05 from '../assets/settings/writing/writing-arc-05.png';
import writingArc06 from '../assets/settings/writing/writing-arc-06.png';
import writingArc07 from '../assets/settings/writing/writing-arc-07.png';
import writingArc08 from '../assets/settings/writing/writing-arc-08.png';
import writingArc09 from '../assets/settings/writing/writing-arc-09.png';
import writingArc10 from '../assets/settings/writing/writing-arc-10.png';
import writingArc11 from '../assets/settings/writing/writing-arc-11.png';
import writingArc12 from '../assets/settings/writing/writing-arc-12.png';
import writingArc13 from '../assets/settings/writing/writing-arc-13.png';
import writingArc14 from '../assets/settings/writing/writing-arc-14.png';
import writingArc15 from '../assets/settings/writing/writing-arc-15.png';
import writingArc16 from '../assets/settings/writing/writing-arc-16.png';
import writingArc17 from '../assets/settings/writing/writing-arc-17.png';
import writingArc18 from '../assets/settings/writing/writing-arc-18.png';
import writingArc19 from '../assets/settings/writing/writing-arc-19.png';
import writingArc20 from '../assets/settings/writing/writing-arc-20.png';
import writingArc21 from '../assets/settings/writing/writing-arc-21.png';
import writingArc22 from '../assets/settings/writing/writing-arc-22.png';
import writingArc23 from '../assets/settings/writing/writing-arc-23.png';
import writingArc24 from '../assets/settings/writing/writing-arc-24.png';
import writingCheck from '../assets/settings/writing/writing-check.png';
import dlBase from '../assets/settings/downloads/dl-base.png';
import dlDotOff1 from '../assets/settings/downloads/dl-dot-off-1.png';
import dlDotOff2 from '../assets/settings/downloads/dl-dot-off-2.png';
import dlDotOff3 from '../assets/settings/downloads/dl-dot-off-3.png';
import dlArrow from '../assets/settings/downloads/dl-arrow.png';
import dlArrowGrey from '../assets/settings/downloads/dl-arrow-grey.png';
import prBase from '../assets/settings/profiles/pr-base.png';
import prArcOff1 from '../assets/settings/profiles/pr-arc-off-1.png';
import prArcOff2 from '../assets/settings/profiles/pr-arc-off-2.png';
import prArcOff3 from '../assets/settings/profiles/pr-arc-off-3.png';
import profilesCardBase from '../assets/settings/profiles/profiles-card-base.png';
import profilesPlus from '../assets/settings/profiles/profiles-plus.png';
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
}

interface HeroArtSpec {
  canvas: { w: number; h: number };
  layers: readonly HeroLayer[];
}

// Source px of the prototype art; the canvas is the visible art box, so soft edges may overhang it.
export const HERO_ART: Record<HeroId, HeroArtSpec> = {
  general: {
    canvas: { w: 865, h: 734 },
    layers: [
      { id: 'base', src: generalBaseNoknob, x: -3, y: -3, w: 871, h: 765 },
      { id: 'track', src: generalTrackGrey, x: 303, y: 468, w: 272, h: 167 },
      { id: 'disc', src: generalDiscDim, x: 458, y: 129, w: 349, h: 430 },
      { id: 'knob', src: generalKnob, x: 422, y: 507, w: 138, h: 134 },
    ],
  },
  writing: {
    canvas: { w: 324, h: 309 },
    layers: [
      { id: 'base', src: writingBase, x: -2, y: -1, w: 338, h: 316 },
      { id: 'grey', src: writingShieldGrey, x: 211, y: 169, w: 114, h: 141 },
      { id: 'arc01', src: writingArc01, x: 106, y: 12, w: 66, h: 73 },
      { id: 'arc02', src: writingArc02, x: 112, y: 0, w: 62, h: 77 },
      { id: 'arc03', src: writingArc03, x: 116, y: -1, w: 58, h: 77 },
      { id: 'arc04', src: writingArc04, x: 125, y: -1, w: 51, h: 76 },
      { id: 'arc05', src: writingArc05, x: 137, y: -1, w: 39, h: 76 },
      { id: 'arc06', src: writingArc06, x: 149, y: 0, w: 31, h: 63 },
      { id: 'arc07', src: writingArc07, x: 161, y: 1, w: 26, h: 72 },
      { id: 'arc08', src: writingArc08, x: 172, y: 3, w: 24, h: 70 },
      { id: 'arc09', src: writingArc09, x: 180, y: 6, w: 25, h: 66 },
      { id: 'arc10', src: writingArc10, x: 182, y: 10, w: 31, h: 62 },
      { id: 'arc11', src: writingArc11, x: 184, y: 14, w: 36, h: 58 },
      { id: 'arc12', src: writingArc12, x: 186, y: 19, w: 41, h: 54 },
      { id: 'arc13', src: writingArc13, x: 188, y: 24, w: 45, h: 49 },
      { id: 'arc14', src: writingArc14, x: 190, y: 30, w: 49, h: 44 },
      { id: 'arc15', src: writingArc15, x: 193, y: 36, w: 51, h: 38 },
      { id: 'arc16', src: writingArc16, x: 197, y: 42, w: 52, h: 33 },
      { id: 'arc17', src: writingArc17, x: 202, y: 49, w: 51, h: 28 },
      { id: 'arc18', src: writingArc18, x: 208, y: 57, w: 49, h: 23 },
      { id: 'arc19', src: writingArc19, x: 217, y: 65, w: 43, h: 24 },
      { id: 'arc20', src: writingArc20, x: 230, y: 74, w: 33, h: 25 },
      { id: 'arc21', src: writingArc21, x: 229, y: 83, w: 35, h: 27 },
      { id: 'arc22', src: writingArc22, x: 229, y: 91, w: 36, h: 31 },
      { id: 'arc23', src: writingArc23, x: 229, y: 97, w: 36, h: 38 },
      { id: 'arc24', src: writingArc24, x: 204, y: 104, w: 61, h: 46 },
      { id: 'check', src: writingCheck, x: 232, y: 214, w: 68, h: 60 },
    ],
  },
  downloads: {
    canvas: { w: 682, h: 854 },
    layers: [
      { id: 'base', src: dlBase, x: -3, y: 202, w: 688, h: 658 },
      { id: 'dot1', src: dlDotOff1, x: 40, y: 346, w: 71, h: 76 },
      { id: 'dot2', src: dlDotOff2, x: 40, y: 496, w: 69, h: 74 },
      { id: 'dot3', src: dlDotOff3, x: 40, y: 647, w: 70, h: 76 },
      { id: 'arrow', src: dlArrow, x: 206, y: -2, w: 286, h: 285 },
      { id: 'grey', src: dlArrowGrey, x: 206, y: -2, w: 287, h: 281 },
    ],
  },
  profiles: {
    canvas: { w: 1256, h: 868 },
    layers: [
      { id: 'base', src: prBase, x: -3, y: -3, w: 1262, h: 874 },
      { id: 'arc1', src: prArcOff1, x: 593, y: 165, w: 139, h: 81 },
      { id: 'arc2', src: prArcOff2, x: 542, y: 82, w: 245, h: 109 },
      { id: 'arc3', src: prArcOff3, x: 494, y: -3, w: 347, h: 134 },
    ],
  },
  profilesEmpty: {
    canvas: { w: 260, h: 256 },
    layers: [
      { id: 'card', src: profilesCardBase, x: -1, y: -1, w: 281, h: 264 },
      { id: 'plus', src: profilesPlus, x: 127, y: 100, w: 101, h: 109 },
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

// Longest entrance (Writing check pop) plus slack; afterwards state changes run on their transitions.
export const HERO_SETTLE_MS = 1600;
