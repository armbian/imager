// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState, type SyntheticEvent } from 'react';
import { UI } from '../../config';
import { logoFit } from '../../utils/imageFit';
import { vendorInitials } from '../../utils';

interface VendorLogoProps {
  name: string;
  src: string | null;
  size?: 'md' | 'sm' | 'xs';
}

/** A brand mark on a uniform light plate, trimmed to its ink and sized by area; initials until it loads or when it has none. */
export function VendorLogo({ name, src, size = 'md' }: VendorLogoProps) {
  const [fit, setFit] = useState<{ src: string; transform: string | null; light: boolean } | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const ready = !!src && fit?.src === src;

  const measure = (event: SyntheticEvent<HTMLImageElement>) => {
    if (!src) return;
    const result = logoFit(event.currentTarget, UI.BOARD_SHEET.LOGO_AREA);
    setFit({ src, transform: result?.transform ?? null, light: !!result?.light });
  };

  const showImage = !!src && failed !== src;
  return (
    <span
      className={`vlogo vlogo--${size}${ready ? ' is-ready' : ''}${ready && fit.light ? ' is-light' : ''}${showImage ? '' : ' is-initials'}`}
      data-initials={vendorInitials(name)}
      aria-hidden="true"
    >
      {showImage && (
        <img
          src={src}
          alt=""
          draggable={false}
          style={ready && fit.transform ? { transform: fit.transform } : undefined}
          onLoad={measure}
          onError={() => setFailed(src)}
        />
      )}
    </span>
  );
}
