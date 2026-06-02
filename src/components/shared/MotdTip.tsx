import { useState, useEffect, useRef, useCallback } from 'react';
import { Lightbulb, ExternalLink } from 'lucide-react';
import { openUrl } from '../../hooks/useTauri';
import { getShowMotd } from '../../hooks/useSettings';
import { LINKS, TIMING, EVENTS } from '../../config';

interface MotdMessage {
  message: string;
  url: string;
  expiration: string;
}

export function MotdTip() {
  const [tip, setTip] = useState<MotdMessage | null>(null);
  const [showMotd, setShowMotd] = useState<boolean | null>(null);
  const messagesRef = useRef<MotdMessage[]>([]);
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
        const messages: MotdMessage[] = await response.json();

        if (!isMounted) return;

        const now = new Date();
        const validMessages = messages.filter((msg) => {
          if (!msg.expiration) return true;
          const expDate = new Date(msg.expiration);
          return isNaN(expDate.getTime()) || expDate > now;
        });

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

  const handleClick = () => {
    openUrl(tip.url).catch(console.error);
  };

  return (
    <button className="rail-tip" onClick={handleClick} title={tip.message}>
      <span className="rail-tip__node">
        <Lightbulb size={12} />
      </span>
      {/* key on the message so it re-fades each rotation */}
      <span key={tip.message} className="rail-tip__msg">
        {tip.message}
      </span>
      <ExternalLink size={13} className="rail-tip__arrow" />
    </button>
  );
}
