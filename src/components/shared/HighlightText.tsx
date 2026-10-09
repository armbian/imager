// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { splitMatch } from '../../utils';

/** Text with the first match of a search needle marked. */
export function HighlightText({ text, needle }: { text: string; needle: string }) {
  const parts = splitMatch(text, needle);
  if (!parts) return <>{text}</>;
  return (
    <>
      {parts[0]}
      <mark className="text-match">{parts[1]}</mark>
      {parts[2]}
    </>
  );
}
