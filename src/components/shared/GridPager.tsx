interface GridPagerProps {
  pageCount: number;
  page: number;
  onChange: (page: number) => void;
}

/** Windowed page list with ellipses, e.g. [1,'gap',4,5,6,'gap',12]; always keeps
 * first/last page plus a one-page window around current. */
function buildPageList(current: number, count: number): Array<number | 'gap'> {
  const out: Array<number | 'gap'> = [];
  for (let p = 1; p <= count; p++) {
    if (p === 1 || p === count || (p >= current - 1 && p <= current + 1)) {
      out.push(p);
    } else if (out[out.length - 1] !== 'gap') {
      out.push('gap');
    }
  }
  return out;
}

/** Floating frosted pill of numbered page buttons; renders nothing for a single page. */
export function GridPager({ pageCount, page, onChange }: GridPagerProps) {
  if (pageCount <= 1) return null;
  return (
    <nav className="mfr-pager">
      {buildPageList(page, pageCount).map((item, i) =>
        item === 'gap' ? (
          <span key={`gap-${i}`} className="mfr-pager__gap" aria-hidden="true">
            …
          </span>
        ) : (
          <button
            key={item}
            type="button"
            className={`mfr-pager__page${item === page ? ' is-active' : ''}`}
            onClick={() => onChange(item)}
            aria-current={item === page ? 'page' : undefined}
          >
            {item}
          </button>
        )
      )}
    </nav>
  );
}
