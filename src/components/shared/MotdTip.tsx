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

    // Move to next message (cycling through)
    currentIndexRef.current = (currentIndexRef.current + 1) % messagesRef.current.length;
    setTip(messagesRef.current[currentIndexRef.current]);
  }, []);

  useEffect(() => {
    let isMounted = true;

    const fetchMotd = async () => {
      try {
        // Load MOTD preference
        const motdEnabled = await getShowMotd();

        if (!isMounted) return;

        setShowMotd(motdEnabled);

        // Clear any existing interval
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }

        if (!motdEnabled) {
          setTip(null); // Clear the tip
          return; // Don't fetch MOTD if disabled
        }

        const response = await fetch(LINKS.MOTD);
        const messages: MotdMessage[] = await response.json();

        if (!isMounted) return;

        // Filter out expired messages
        const now = new Date();
        const validMessages = messages.filter((msg) => {
          if (!msg.expiration) return true;
          const expDate = new Date(msg.expiration);
          return isNaN(expDate.getTime()) || expDate > now;
        });

        if (validMessages.length > 0) {
          messagesRef.current = validMessages;
          // Pick a random starting message
          currentIndexRef.current = Math.floor(Math.random() * validMessages.length);
          setTip(validMessages[currentIndexRef.current]);

          // Start rotation interval
          intervalRef.current = setInterval(pickNextMessage, TIMING.MOTD_ROTATION);
        }
      } catch (err) {
        console.error('Failed to fetch MOTD:', err);
      }
    };

    fetchMotd();

    // Listen for MOTD setting changes only
    const handleMotdChange = () => {
      fetchMotd();
    };

    window.addEventListener(EVENTS.MOTD_CHANGED, handleMotdChange);

    return () => {
      isMounted = false;
      // Cleanup interval and event listener
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      window.removeEventListener(EVENTS.MOTD_CHANGED, handleMotdChange);
    };
  }, [pickNextMessage]);

  // Don't render if we haven't loaded the setting yet, if MOTD is disabled, or if no tip
  if (showMotd !== true || !tip) {
    return null;
  }

  const handleClick = () => {
    openUrl(tip.url).catch(console.error);
  };

  return (
    <button className="motd-tip" onClick={handleClick}>
      <Lightbulb size={16} className="motd-icon" />
      <span className="motd-message">{tip.message}</span>
      <ExternalLink size={14} className="motd-arrow" />
    </button>
  );
}
