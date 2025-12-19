import { useRef, useEffect, useState } from 'react';

interface MarqueeTextProps {
  text: string;
  maxWidth?: number;
  className?: string;
}

// Component for text that scrolls automatically if it overflows
export function MarqueeText({ text, maxWidth = 180, className = '' }: MarqueeTextProps) {
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

      const overflow = singleTextWidth > maxWidth;
      setIsOverflow(overflow);

      if (overflow) {
        const separatorWidth = 5;
        const scrollDistance = singleTextWidth + separatorWidth;
        const totalWidth = scrollDistance * 2;
        setScrollPercent((scrollDistance / totalWidth) * 100);
      }
    };

    const timer = setTimeout(checkOverflow, 50);
    window.addEventListener('resize', checkOverflow);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('resize', checkOverflow);
    };
  }, [text, maxWidth]);

  return (
    <span
      ref={containerRef}
      className={`marquee-container ${isOverflow ? 'overflow' : ''} ${className}`}
      style={{ maxWidth: `${maxWidth}px` }}
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
