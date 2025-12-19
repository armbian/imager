import { useState, useEffect, useRef, useCallback } from 'react';
import { Lightbulb, ExternalLink } from 'lucide-react';
import { openUrl } from '../../hooks/useTauri';

// Configuration
const MOTD_URL = 'https://raw.githubusercontent.com/armbian/os/main/motd.json';
const ROTATE_INTERVAL_MS = 30000; // 30 seconds

interface MotdMessage {
  message: string;
  url: string;
  expiration: string;
}

export function MotdTip() {
  const [tip, setTip] = useState<MotdMessage | null>(null);
  const messagesRef = useRef<MotdMessage[]>([]);
  const currentIndexRef = useRef(0);

  const pickNextMessage = useCallback(() => {
    if (messagesRef.current.length === 0) return;

    // Move to next message (cycling through)
    currentIndexRef.current = (currentIndexRef.current + 1) % messagesRef.current.length;
    setTip(messagesRef.current[currentIndexRef.current]);
  }, []);

  useEffect(() => {
    const fetchMotd = async () => {
      try {
        const response = await fetch(MOTD_URL);
        const messages: MotdMessage[] = await response.json();

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
        }
      } catch (err) {
        console.error('Failed to fetch MOTD:', err);
      }
    };

    fetchMotd();

    // Rotate messages every 30 seconds
    const interval = setInterval(pickNextMessage, ROTATE_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [pickNextMessage]);

  if (!tip) return null;

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
