import { useState } from 'react';
import { BOARD_LOGO_LIGHT, BOARD_LOGO_DARK } from '../../assets';

interface BoardImageProps {
  src?: string | null;
  alt: string;
  className?: string;
}

// Board photo with a faded Armbian wordmark watermark fallback when missing or it errors.
export function BoardImage({ src, alt, className }: BoardImageProps) {
  const [failed, setFailed] = useState(false);
  const [lastSrc, setLastSrc] = useState(src);

  // Reset error flag during render on src change, avoiding a setState-in-effect cascade
  if (src !== lastSrc) {
    setLastSrc(src);
    setFailed(false);
  }

  if (!src || failed) {
    return (
      <span className={(className ? className + ' ' : '') + 'board-image-fallback'}>
        <img className="board-image-fallback__logo is-light" src={BOARD_LOGO_LIGHT} alt="" />
        <img className="board-image-fallback__logo is-dark" src={BOARD_LOGO_DARK} alt="" />
      </span>
    );
  }

  return <img src={src} alt={alt} className={className} onError={() => setFailed(true)} />;
}
