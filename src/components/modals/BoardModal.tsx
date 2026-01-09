import { useState, useEffect, useMemo, useRef } from 'react';
import { Download, Crown, Shield, Users, Clock, Tv, Wrench } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay, BoardCardSkeleton, SearchBox } from '../shared';
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
  const [showSkeleton, setShowSkeleton] = useState(false);
  const loadedSlugsRef = useRef<Set<string>>(new Set());

  // Use hook for async data fetching
  const { data: boards, loading, error, reload } = useAsyncDataWhen<BoardInfo[]>(
    isOpen,
    () => getBoards(),
    [isOpen]
  );

  // Use shared hook for vendor logo validation
  const { isLoaded: vendorLogosChecked, getEffectiveVendor } = useVendorLogos(boards, isOpen);

  // Derive boards ready state from data availability
  const boardsReady = useMemo(() => {
    return boards && boards.length > 0 && vendorLogosChecked;
  }, [boards, vendorLogosChecked]);

  // Show skeleton with minimum delay
  useEffect(() => {
    let skeletonTimeout: NodeJS.Timeout;

    if (loading) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Show skeleton during loading
      setShowSkeleton(true);
    } else if (boardsReady) {
      // Keep skeleton visible for at least 300ms
      skeletonTimeout = setTimeout(() => {
        setShowSkeleton(false);
      }, 300);
    }

    return () => {
      if (skeletonTimeout) {
        clearTimeout(skeletonTimeout);
      }
    };
  }, [loading, boardsReady]);

  // Reset images when manufacturer changes
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- Reset state when manufacturer changes
    setBoardImages({});
    loadedSlugsRef.current.clear();
  }, [manufacturer?.id]);

  // Pre-load images for current manufacturer
  useEffect(() => {
    const manufacturerId = manufacturer?.id;
    if (!isOpen || !manufacturerId || !boards || !vendorLogosChecked) return;

    const manufacturerBoards = boards.filter((board) => {
      return getEffectiveVendor(board) === manufacturerId;
    });

    if (manufacturerBoards.length === 0) return;

    const loadImages = async () => {
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
    };

    loadImages();
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
      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : (
        <div className="board-grid-container">
          {showSkeleton && <BoardCardSkeleton count={12} />}
          {!showSkeleton && filteredBoards.map((board) => (
            <button
              key={board.slug}
              className="board-grid-item"
              onClick={() => onSelect(board)}
            >
              <span className={`badge-image-count ${!boardsReady ? 'skeleton' : ''}`}>
                <Download size={10} />{board.image_count}
              </span>
              <div className="board-grid-image">
                {boardsReady && board.slug in boardImages ? (
                  <img
                    src={boardImages[board.slug] ?? fallbackImage}
                    alt={board.name}
                    className={boardImages[board.slug] ? '' : 'fallback-image'}
                    onError={(e) => {
                      const img = e.currentTarget;
                      if (img.src !== fallbackImage) {
                        img.src = fallbackImage;
                        img.className = 'fallback-image';
                      }
                    }}
                  />
                ) : (
                  <div className="skeleton" style={{ width: '100px', height: '100px', borderRadius: '8px' }} />
                )}
              </div>
              <div className="board-grid-info">
                {boardsReady ? (
                  <div className="board-grid-name">{board.name}</div>
                ) : (
                  <div className="skeleton" style={{ width: '80%', height: '14px', marginBottom: '8px' }} />
                )}
                <div className="board-grid-badges">
                  {boardsReady ? (
                    <>
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
                    </>
                  ) : (
                    <>
                      <div className="skeleton" style={{ width: '50px', height: '18px' }} />
                      <div className="skeleton" style={{ width: '50px', height: '18px' }} />
                    </>
                  )}
                </div>
              </div>
            </button>
          ))}
          {filteredBoards.length === 0 && !showSkeleton && (
            <div className="no-results">
              <p>{t('modal.noBoards')}</p>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
