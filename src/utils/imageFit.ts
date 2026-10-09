// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { UI } from '../config';

const { ALPHA_MIN, MAX_SCALE, SAMPLE_MAX, LIGHT_INK, PLATE_COVERAGE } = UI.IMAGE_FIT;

/** Opaque content of an image as fractions of its natural size, and whether that content is light ink */
interface OpaqueBounds {
  x: number;
  y: number;
  w: number;
  h: number;
  light: boolean;
}

const boundsCache = new Map<string, OpaqueBounds | null>();

function opaqueBounds(img: HTMLImageElement): OpaqueBounds | null {
  const key = img.currentSrc || img.src;
  const cached = boundsCache.get(key);
  if (cached !== undefined) return cached;

  let result: OpaqueBounds | null = null;
  const { naturalWidth, naturalHeight } = img;
  const sample = Math.min(1, SAMPLE_MAX / Math.max(naturalWidth, naturalHeight, 1));
  const w = Math.max(1, Math.round(naturalWidth * sample));
  const h = Math.max(1, Math.round(naturalHeight * sample));
  try {
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (ctx && naturalWidth && naturalHeight) {
      ctx.drawImage(img, 0, 0, w, h);
      const data = ctx.getImageData(0, 0, w, h).data;
      let minX = w;
      let minY = h;
      let maxX = -1;
      let maxY = -1;
      let ink = 0;
      let lum = 0;
      for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
          const i = (y * w + x) * 4;
          if (data[i + 3] <= ALPHA_MIN) continue;
          ink++;
          lum += (0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2]) / 255;
          if (x < minX) minX = x;
          if (x > maxX) maxX = x;
          if (y < minY) minY = y;
          if (y > maxY) maxY = y;
        }
      }
      if (ink > 0) {
        const bw = maxX - minX + 1;
        const bh = maxY - minY + 1;
        result = {
          x: minX / w,
          y: minY / h,
          w: bw / w,
          h: bh / h,
          light: lum / ink > LIGHT_INK && ink / (bw * bh) < PLATE_COVERAGE,
        };
      }
    }
  } catch {
    result = null;
  }
  boundsCache.set(key, result);
  return result;
}

// The image box is the element itself, drawn with `object-fit: contain`; the transform scales and recentres it.
function transformFor(img: HTMLImageElement, b: OpaqueBounds, scaleFor: (cw: number, ch: number, bw: number, bh: number) => number) {
  const { naturalWidth: w, naturalHeight: h } = img;
  const boxW = img.clientWidth || Math.max(w, h);
  const boxH = img.clientHeight || Math.max(w, h);
  const s0 = Math.min(boxW / w, boxH / h);
  const cw = b.w * w * s0;
  const ch = b.h * h * s0;
  const scale = Math.min(MAX_SCALE, scaleFor(cw, ch, boxW, boxH));
  const dx = (b.x + b.w / 2 - 0.5) * ((w * s0) / boxW) * 100;
  const dy = (b.y + b.h / 2 - 0.5) * ((h * s0) / boxH) * 100;
  return `scale(${scale.toFixed(3)}) translate(${(-dx).toFixed(2)}%, ${(-dy).toFixed(2)}%)`;
}

/** Grows a contained image so its opaque content, not its uneven transparent margins, spans `fill` of the box; null if unreadable */
export function opaqueFitTransform(img: HTMLImageElement, fill: number): string | null {
  const b = opaqueBounds(img);
  if (!b) return null;
  return transformFor(img, b, (cw, ch, boxW, boxH) => Math.min((fill * boxW) / cw, (fill * boxH) / ch));
}

/** Sizes a logo by its mark's area so wordmarks and emblems weigh the same, and flags light ink; null if unreadable */
export function logoFit(img: HTMLImageElement, area: number): { transform: string; light: boolean } | null {
  const b = opaqueBounds(img);
  if (!b) return null;
  const transform = transformFor(img, b, (cw, ch, boxW, boxH) => {
    const aspect = cw / ch;
    let width = Math.min(Math.sqrt(area * boxW * boxH * aspect), boxW);
    if (width / aspect > boxH) width = boxH * aspect;
    return width / cw;
  });
  return { transform, light: b.light };
}
