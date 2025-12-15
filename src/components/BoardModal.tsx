import { useState, useEffect, useMemo, useRef } from 'react';
import { Search, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay } from './shared/ErrorDisplay';
import type { BoardInfo, Manufacturer } from '../types';
import { getBoards, getBoardImageUrl } from '../hooks/useTauri';
import { useAsyncDataWhen } from '../hooks/useAsyncData';
import { getManufacturer } from '../config';
import fallbackImage from '../assets/armbian-logo_nofound.png';

interface BoardModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (board: BoardInfo) => void;
  manufacturer: Manufacturer | null;
}

export function BoardModal({ isOpen, onClose, onSelect, manufacturer }: BoardModalProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState('');
  const [boardImages, setBoardImages] = useState<Record<string, string | null>>({});
  const [imagesReady, setImagesReady] = useState(false);
  const loadedSlugsRef = useRef<Set<string>>(new Set());

  // Use hook for async data fetching
  const { data: boards, loading, error, reload } = useAsyncDataWhen<BoardInfo[]>(
    isOpen,
    () => getBoards(),
    [isOpen]
  );

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- Reset state when manufacturer changes
    setSearch('');
    setImagesReady(false);
  }, [manufacturer]);

  // Pre-load images for current manufacturer
  useEffect(() => {
    const manufacturerId = manufacturer?.id;
    if (!isOpen || !manufacturerId || !boards) return;

    const manufacturerBoards = boards.filter((board) => {
      const mfr = getManufacturer(board.slug, board.name);
      return mfr === manufacturerId;
    });

    if (manufacturerBoards.length === 0) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Early return case
      setImagesReady(true);
      return;
    }

    let cancelled = false;

    const loadImages = async () => {
      setImagesReady(false);

      await Promise.all(manufacturerBoards.map(async (board) => {
        if (loadedSlugsRef.current.has(board.slug)) return;

        const url = await getBoardImageUrl(board.slug);
        if (!url) {
          loadedSlugsRef.current.add(board.slug);
          setBoardImages((prev) => ({ ...prev, [board.slug]: null }));
          return;
        }

        // Pre-load in browser
        await new Promise<void>((resolve) => {
          const img = new Image();
          img.onload = () => {
            loadedSlugsRef.current.add(board.slug);
            setBoardImages((prev) => ({ ...prev, [board.slug]: url }));
            resolve();
          };
          img.onerror = () => {
            loadedSlugsRef.current.add(board.slug);
            setBoardImages((prev) => ({ ...prev, [board.slug]: null }));
            resolve();
          };
          img.src = url;
        });
      }));

      if (!cancelled) {
        setImagesReady(true);
      }
    };

    loadImages();

    return () => { cancelled = true; };
  }, [isOpen, manufacturer?.id, boards]);

  const filteredBoards = useMemo(() => {
    if (!manufacturer || !boards) return [];
    const searchLower = search.toLowerCase();
    return boards
      .filter((board) => {
        const mfr = getManufacturer(board.slug, board.name);
        if (mfr !== manufacturer.id) return false;
        return board.name.toLowerCase().includes(searchLower) ||
          board.slug.toLowerCase().includes(searchLower);
      })
      .sort((a, b) => {
        if (a.has_promoted && !b.has_promoted) return -1;
        if (!a.has_promoted && b.has_promoted) return 1;
        return a.name.localeCompare(b.name);
      });
  }, [boards, manufacturer, search]);

  const title = manufacturer ? `${manufacturer.name} ${t('home.boards')}` : t('modal.selectBoard');

  const searchBarContent = (
    <div className="modal-search">
      <div className="search-box" style={{ marginBottom: 0 }}>
        <Search className="search-icon" size={18} />
        <input
          type="text"
          placeholder={t('modal.searchBoard')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="search-input"
          autoFocus
        />
      </div>
    </div>
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title} searchBar={searchBarContent}>
      {loading || !imagesReady ? (
        <div className="loading">
          <div className="spinner" />
          <p>{t('modal.loading')}</p>
        </div>
      ) : error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : (
        <div className="board-grid-container">
          {filteredBoards.map((board) => (
            <button
              key={board.slug}
              className="board-grid-item"
              onClick={() => onSelect(board)}
            >
              <span className="badge-image-count"><Download size={10} />{board.image_count}</span>
              <div className="board-grid-image">
                <img
                  src={boardImages[board.slug] || fallbackImage}
                  alt={board.name}
                  className={!boardImages[board.slug] ? 'fallback-image' : ''}
                  onError={(e) => {
                    const img = e.currentTarget;
                    if (img.src !== fallbackImage) {
                      img.src = fallbackImage;
                      img.className = 'fallback-image';
                    }
                  }}
                />
              </div>
              <div className="board-grid-info">
                <div className="board-grid-name">{board.name}</div>
                {board.has_promoted && <span className="badge-recommended">{t('modal.promoted')}</span>}
              </div>
            </button>
          ))}
          {filteredBoards.length === 0 && (
            <div className="no-results">
              <p>{t('modal.noBoards')}</p>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
