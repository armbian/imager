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
  const [isOverflow, setIsOverflow] = useState(false);
  const [scrollPercent, setScrollPercent] = useState(50);

  useEffect(() => {
    const checkOverflow = () => {
      if (!containerRef.current) return;

      const computedStyle = window.getComputedStyle(containerRef.current);

      const measureSpan = document.createElement('span');
      measureSpan.style.cssText = `
        position: absolute;
        visibility: hidden;
        white-space: nowrap;
        font-family: ${computedStyle.fontFamily};
        font-size: ${computedStyle.fontSize};
        font-weight: ${computedStyle.fontWeight};
        letter-spacing: ${computedStyle.letterSpacing};
        text-transform: ${computedStyle.textTransform};
      `;
      measureSpan.textContent = text;

      let singleTextWidth = 0;
      try {
        document.body.appendChild(measureSpan);
        singleTextWidth = measureSpan.offsetWidth;
      } finally {
        if (measureSpan.parentNode) {
          measureSpan.parentNode.removeChild(measureSpan);
        }
      }

      // Compare against the actual available width when responsive, else the cap.
      const available = responsive ? containerRef.current.clientWidth : maxWidth;
      const overflow = available > 0 && singleTextWidth > available;
      setIsOverflow(overflow);

      if (overflow) {
        const scrollDistance = singleTextWidth + UI.MARQUEE.SEPARATOR_WIDTH;
        const totalWidth = scrollDistance * 2;
        setScrollPercent((scrollDistance / totalWidth) * 100);
      }
    };

    const timer = setTimeout(checkOverflow, 50);
    window.addEventListener('resize', checkOverflow);

    // In responsive mode the column width can change without a window resize
    // (sidebar/panel reflow), so observe the container itself.
    let observer: ResizeObserver | undefined;
    if (responsive && containerRef.current && typeof ResizeObserver !== 'undefined') {
      observer = new ResizeObserver(checkOverflow);
      observer.observe(containerRef.current);
    }

    return () => {
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
      <span
        className="marquee-content"
        style={isOverflow ? { '--scroll-percent': `-${scrollPercent}%` } as React.CSSProperties : undefined}
      >
        {text}
        {isOverflow && <>&nbsp;{text}</>}
      </span>
    </span>
  );
}
