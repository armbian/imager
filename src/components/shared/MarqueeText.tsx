import { useRef, useEffect, useState } from 'react';
import { UI } from '../../config';

interface MarqueeTextProps {
  text: string;
  /** Fixed overflow threshold in px; omitted = measure own width and scroll when
   * text exceeds available space (responsive mode). */
  maxWidth?: number;
  className?: string;
}

// Component for text that scrolls automatically if it overflows
export function MarqueeText({ text, maxWidth, className = '' }: MarqueeTextProps) {
  // No explicit cap → measure the real container width and react to layout changes.
  const responsive = maxWidth === undefined;
  const containerRef = useRef<HTMLSpanElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [isOverflow, setIsOverflow] = useState(false);
  const [shiftPx, setShiftPx] = useState(0);

  useEffect(() => {
    let active = true;

    const checkOverflow = () => {
      const container = containerRef.current;
      const measure = measureRef.current;
      if (!active || !container || !measure) return;

      // The hidden in-container span inherits the real rendered font, so its width
      // is exact even across font swaps (a detached styled clone is not).
      const textWidth = measure.offsetWidth;
      const available = responsive ? container.clientWidth : maxWidth;
      const overflow = available > 0 && textWidth > available + 2;
      setIsOverflow(overflow);

      if (overflow) {
        // Exact pixel shift: copy 2 lands where copy 1 started, so the loop is seamless.
        setShiftPx(textWidth + UI.MARQUEE.SEPARATOR_WIDTH);
      }
    };

    const timer = setTimeout(checkOverflow, 50);
    document.fonts?.ready.then(checkOverflow);
    window.addEventListener('resize', checkOverflow);

    // In responsive mode the column width can change without a window resize
    // (sidebar/panel reflow), so observe the container itself.
    let observer: ResizeObserver | undefined;
    if (responsive && containerRef.current && typeof ResizeObserver !== 'undefined') {
      observer = new ResizeObserver(checkOverflow);
      observer.observe(containerRef.current);
    }

    return () => {
      active = false;
      clearTimeout(timer);
      window.removeEventListener('resize', checkOverflow);
      observer?.disconnect();
    };
  }, [text, maxWidth, responsive]);

  return (
    <span
      ref={containerRef}
      className={`marquee-container ${isOverflow ? 'overflow' : ''} ${className}`}
      style={{ maxWidth: responsive ? '100%' : `${maxWidth}px` }}
      title={text}
    >
      <span ref={measureRef} className="marquee-measure" aria-hidden="true">
        {text}
      </span>
      <span
        className="marquee-content"
        style={isOverflow ? { '--marquee-shift': `-${shiftPx}px` } as React.CSSProperties : undefined}
      >
        {text}
        {isOverflow && (
          <>
            <span
              className="marquee-spacer"
              aria-hidden="true"
              style={{ width: UI.MARQUEE.SEPARATOR_WIDTH }}
            />
            {text}
          </>
        )}
      </span>
    </span>
  );
}
