// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

const GITHUB_ISSUE_URL = / https:\/\/github\.com\/[^/]+\/[^/]+\/(?:pull|issues)\/(\d+)/g;
const GITHUB_ISSUE_LINK = /\[[^\]]+\]\(https:\/\/github\.com\/[^/]+\/[^/]+\/(?:pull|issues)\/(\d+)\)/g;
const FULL_CHANGELOG = /\*{0,2}Full Changelog\*{0,2}: https:\/\/github\.com\/[^\s]+/gi;
const LIST_ITEM = /^\s*[-*]\s+/;
const MENTION = /@([a-zA-Z0-9_-]+)/g;

function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function formatInline(text: string): string {
  return (
    escapeHtml(
      text
        .replace(GITHUB_ISSUE_URL, ' #PR$1 ')
        .replace(GITHUB_ISSUE_LINK, '#PR$1')
    )
      .replace(/#PR(\d+)/g, '<span class="pr-number">#$1</span>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*([^*]+)\*/g, '<em>$1</em>')
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/\[([^\]]+)\]\(https:\/\/github\.com\/[^)]+\)/g, '$1')
      .replace(/https:\/\/github\.com\/[^\s]+/g, '')
      .replace(/,\s*$/g, '')
      .replace(/\s+/g, ' ')
      .trim()
      // GitHub links collapse to their text; other external links stay clickable
      .replace(
        /\[([^\]]+)\]\((?!https?:\/\/github\.com)(https?:\/\/[^)]+)\)/g,
        '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>'
      )
  );
}

/** Renders a GitHub release body (markdown subset) to HTML; everything taken from the body is escaped. */
export function releaseNotesHtml(body: string): string {
  let html = '';
  let inList = false;
  let inCodeBlock = false;

  const closeList = () => {
    if (inList) html += '</ul>';
    inList = false;
  };

  for (const line of body.replace(FULL_CHANGELOG, '').split('\n')) {
    if (line.trim().startsWith('```')) {
      inCodeBlock = !inCodeBlock;
      continue;
    }
    if (inCodeBlock) {
      html += `<div class="code-block">${escapeHtml(line)}</div>`;
      continue;
    }
    if (LIST_ITEM.test(line)) {
      if (!inList) html += '<ul>';
      inList = true;
      html += `<li>${formatInline(line.replace(LIST_ITEM, ''))}</li>`;
      continue;
    }
    closeList();
    // Blocks carry their own spacing, so blank lines add nothing
    if (line.trim() === '') continue;
    if (line.startsWith('## ')) {
      html += `<h2>${escapeHtml(line.slice(3))}</h2>`;
    } else if (line.startsWith('### ')) {
      html += `<h3>${escapeHtml(line.slice(4))}</h3>`;
    } else {
      // A line that was only a GitHub link formats to nothing
      const text = formatInline(line);
      if (text) html += `<p>${text}</p>`;
    }
  }

  closeList();
  return html;
}

/** Unique @mentions in a release body, sorted */
export function releaseContributors(body: string | null): string[] {
  const mentions = body?.match(MENTION);
  if (!mentions) return [];
  return Array.from(new Set(mentions.map((m) => m.substring(1)))).sort();
}
