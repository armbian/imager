// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useAsyncData } from './useAsyncData';
import { getCachedBoardImage } from './useTauri';

// Photos already fetched this session, so a page seen before renders them at once instead of shimmering again
const seen = new Map<string, string>();

/** Board photos by slug for the boards on screen; a slug is absent while it loads, null when it has no photo. */
export function useBoardPhotos(slugs: readonly string[]): Record<string, string | null> {
  const key = slugs.join('\x1f');
  const { data } = useAsyncData<Record<string, string | null>>(
    async () => {
      const photos = await Promise.all(
        slugs.map((slug) => seen.get(slug) ?? getCachedBoardImage(slug).catch(() => null))
      );
      photos.forEach((photo, i) => photo && seen.set(slugs[i], photo));
      return Object.fromEntries(slugs.map((slug, i) => [slug, photos[i]]));
    },
    [key]
  );
  const known: Record<string, string | null> = {};
  for (const slug of slugs) {
    const photo = seen.get(slug);
    if (photo) known[slug] = photo;
  }
  return { ...known, ...data };
}
