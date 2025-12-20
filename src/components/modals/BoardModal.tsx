import { useState, useEffect, useMemo, useRef } from 'react';
import { Download, Crown, Shield, Users, Clock, Tv, Wrench } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay, LoadingState, SearchBox } from '../shared';
import type { BoardInfo, Manufacturer } from '../../types';
import { getBoards, getBoardImageUrl } from '../../hooks/useTauri';
import { useAsyncDataWhen } from '../../hooks/useAsyncData';
import { useVendorLogos } from '../../hooks/useVendorLogos';
import { compareBoardsBySupport, preloadImage } from '../../utils';
import fallbackImage from '../../assets/armbian-logo_nofound.png';

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

  // Use shared hook for vendor logo validation
  const { isLoaded: vendorLogosChecked, getEffectiveVendor } = useVendorLogos(boards, isOpen);

  // Reset state when modal closes or manufacturer changes
  useEffect(() => {
    if (!isOpen) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Reset state on modal close
      setImagesReady(false);
    }
    setSearch('');
  }, [isOpen, manufacturer]);

  // Pre-load images for current manufacturer
  useEffect(() => {
    const manufacturerId = manufacturer?.id;
    if (!isOpen || !manufacturerId || !boards || !vendorLogosChecked) return;

    const manufacturerBoards = boards.filter((board) => {
      return getEffectiveVendor(board) === manufacturerId;
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
        const loaded = await preloadImage(url);
        loadedSlugsRef.current.add(board.slug);
        setBoardImages((prev) => ({ ...prev, [board.slug]: loaded ? url : null }));
      }));

      if (!cancelled) {
        setImagesReady(true);
      }
    };

    loadImages();

    return () => { cancelled = true; };
  }, [isOpen, manufacturer?.id, boards, vendorLogosChecked, getEffectiveVendor]);

  const filteredBoards = useMemo(() => {
    if (!manufacturer || !boards || !vendorLogosChecked) return [];
    const searchLower = search.toLowerCase();
    return boards
      .filter((board) => {
        const vendorId = getEffectiveVendor(board);
        if (vendorId !== manufacturer.id) return false;
        return board.name.toLowerCase().includes(searchLower) ||
          board.slug.toLowerCase().includes(searchLower);
      })
      .sort(compareBoardsBySupport);
  }, [boards, manufacturer, search, vendorLogosChecked, getEffectiveVendor]);

  const title = t('modal.selectBoard');

  const searchBarContent = (
    <SearchBox
      value={search}
      onChange={setSearch}
      placeholder={t('modal.searchBoard')}
    />
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title} searchBar={searchBarContent}>
      <LoadingState isLoading={loading || !imagesReady}>
        {error ? (
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
                  <div className="board-grid-badges">
                    {board.has_platinum_support && (
                      <span className="badge-platinum">
                        <Crown size={10} />
                        <span>Platinum</span>
                      </span>
                    )}
                    {board.has_standard_support && !board.has_platinum_support && (
                      <span className="badge-standard">
                        <Shield size={10} />
                        <span>Standard</span>
                      </span>
                    )}
                    {board.has_community_support && (
                      <span className="badge-community">
                        <Users size={10} />
                        <span>Community</span>
                      </span>
                    )}
                    {board.has_eos_support && (
                      <span className="badge-eos">
                        <Clock size={10} />
                        <span>EOS</span>
                      </span>
                    )}
                    {board.has_tvb_support && (
                      <span className="badge-tvb">
                        <Tv size={10} />
                        <span>TV Box</span>
                      </span>
                    )}
                    {board.has_wip_support && (
                      <span className="badge-wip">
                        <Wrench size={10} />
                        <span>WIP</span>
                      </span>
                    )}
                  </div>
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
      </LoadingState>
    </Modal>
  );
}
