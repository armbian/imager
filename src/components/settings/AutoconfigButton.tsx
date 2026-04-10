import { useState, useEffect } from 'react';
import { Settings2 } from 'lucide-react';
import { AutoconfigModal } from './AutoconfigModal';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { EVENTS } from '../../config';
import { getAutoconfigEnabled } from '../../hooks/useAutoconfig';

export function AutoconfigButton() {
  const [isOpen, setIsOpen] = useState(false);
  const [openCount, setOpenCount] = useState(0);
  const [isEnabled, setIsEnabled] = useState(false);

  useEffect(() => {
    // Refresh enabled state dynamically or on open/close
    getAutoconfigEnabled().then(setIsEnabled);
  }, [isOpen]);

  useEffect(() => {
    const handler = () => setIsOpen(false);
    window.addEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
    return () => window.removeEventListener(EVENTS.CACHE_IMAGE_REUSE, handler);
  }, []);

  const { isExiting, handleClose } = useModalExitAnimation({
    onClose: () => setIsOpen(false),
    duration: 200,
  });

  const handleOpen = () => {
    setOpenCount((c) => c + 1);
    setIsOpen(true);
  };

  return (
    <>
      <button
        className="home-custom-button"
        onClick={handleOpen}
        title="OS Customization (Optional)"
      >
        <div style={{ position: 'relative', display: 'flex', alignItems: 'center' }}>
          <Settings2 size={16} />
          {isEnabled && (
            <span style={{
              position: 'absolute', top: '-4px', right: '-4px',
              width: '6px', height: '6px', borderRadius: '50%',
              backgroundColor: 'var(--color-success)', border: '1px solid var(--bg-primary)'
            }} />
          )}
        </div>
        OS Customization (Optional)
      </button>

      <AutoconfigModal
        key={openCount}
        isOpen={isOpen && !isExiting}
        onClose={handleClose}
      />
    </>
  );
}
