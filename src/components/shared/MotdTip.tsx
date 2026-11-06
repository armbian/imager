// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useState, useEffect, useRef, useCallback } from 'react';
import { ExternalLink } from 'lucide-react';
import { openUrl } from '../../hooks/useTauri';
import { getShowMotd } from '../../hooks/useSettings';
import { LINKS, TIMING, EVENTS, featuredToMotdTips, motdIconFor, type MotdTip as MotdTipData } from '../../config';

export function MotdTip() {
  const [tip, setTip] = useState<MotdTipData | null>(null);
  const [showMotd, setShowMotd] = useState<boolean | null>(null);
  const messagesRef = useRef<MotdTipData[]>([]);
  const currentIndexRef = useRef(0);
  const intervalRef = useRef<NodeJS.Timeout | null>(null);

  const pickNextMessage = useCallback(() => {
    if (messagesRef.current.length === 0) return;

    // Cycle to the next message, wrapping at the end.
    currentIndexRef.current = (currentIndexRef.current + 1) % messagesRef.current.length;
    setTip(messagesRef.current[currentIndexRef.current]);
  }, []);

  useEffect(() => {
    let isMounted = true;

    const fetchMotd = async () => {
      try {
        const motdEnabled = await getShowMotd();

        if (!isMounted) return;

        setShowMotd(motdEnabled);

        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }

        if (!motdEnabled) {
          setTip(null);
          return;
        }

        const response = await fetch(LINKS.MOTD);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const validMessages = featuredToMotdTips(await response.json(), new Date());

        if (!isMounted) return;

        if (validMessages.length > 0) {
          messagesRef.current = validMessages;
          currentIndexRef.current = Math.floor(Math.random() * validMessages.length);
          setTip(validMessages[currentIndexRef.current]);

          intervalRef.current = setInterval(pickNextMessage, TIMING.MOTD_ROTATION);
        }
      } catch (err) {
        console.error('Failed to fetch MOTD:', err);
      }
    };

    fetchMotd();

    const handleMotdChange = () => {
      fetchMotd();
    };

    window.addEventListener(EVENTS.MOTD_CHANGED, handleMotdChange);

    return () => {
      isMounted = false;
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      window.removeEventListener(EVENTS.MOTD_CHANGED, handleMotdChange);
    };
  }, [pickNextMessage]);

  // Don't render if not loaded yet, disabled, or no tip (toggle lives in Settings)
  if (showMotd !== true || !tip) {
    return null;
  }

  const Icon = motdIconFor(tip.type);

  const handleClick = () => {
    openUrl(tip.url).catch(console.error);
  };

  return (
    <button className="rail-tip" onClick={handleClick} title={tip.message}>
      {/* key on the message too: remounting replays the finite halo on each rotation */}
      <span key={`node-${tip.message}`} className="rail-tip__node">
        <Icon size={12} />
      </span>
      {/* key on the message so it re-fades each rotation */}
      <span key={tip.message} className="rail-tip__msg">
        {tip.message}
      </span>
      <ExternalLink size={13} className="rail-tip__arrow" />
    </button>
  );
}
