import { useState, useMemo, useEffect } from 'react';
import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay } from './shared/ErrorDisplay';
import type { BoardInfo, Manufacturer } from '../types';
import { getBoards } from '../hooks/useTauri';
import { useAsyncDataWhen } from '../hooks/useAsyncData';
import {
  MANUFACTURERS,
  getManufacturer,
  getVendorLogoUrl,
  vendorHasLogo,
  type ManufacturerConfig,
} from '../config';

// Re-export Manufacturer type for backward compatibility
export type { Manufacturer } from '../types';

function ManufacturerIcon({ id, config }: { id: string; config: ManufacturerConfig }) {
  const [imageError, setImageError] = useState(false);
  const hasLogo = vendorHasLogo(id);
  const logoUrl = getVendorLogoUrl(id);

  if (!hasLogo || imageError || id === 'other') {
    return (
      <div className="list-item-icon" style={{ backgroundColor: config.color }}>
        {config.name.substring(0, 2).toUpperCase()}
      </div>
    );
  }

  return (
    <div className="list-item-icon manufacturer-logo">
      <img
        src={logoUrl}
        alt={config.name}
        onError={() => setImageError(true)}
      />
    </div>
  );
}

interface ManufacturerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (manufacturer: Manufacturer) => void;
}

export function ManufacturerModal({ isOpen, onClose, onSelect }: ManufacturerModalProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState('');

  // Use hook for async data fetching - only fetch when modal opens and no data yet
  const { data: boards, loading, error, reload } = useAsyncDataWhen<BoardInfo[]>(
    isOpen,
    () => getBoards(),
    [isOpen]
  );

  const manufacturers = useMemo(() => {
    if (!boards) return [];
    const searchLower = search.toLowerCase();
    const counts: Record<string, number> = {};

    for (const board of boards) {
      const mfr = getManufacturer(board.slug, board.name);
      if (!counts[mfr]) counts[mfr] = 0;
      counts[mfr]++;
    }

    const result: Manufacturer[] = Object.entries(counts)
      .filter(([key, count]) => {
        if (count === 0) return false;
        const config = MANUFACTURERS[key];
        return config.name.toLowerCase().includes(searchLower);
      })
      .map(([key, count]) => ({
        id: key,
        name: MANUFACTURERS[key].name,
        color: MANUFACTURERS[key].color,
        boardCount: count,
      }))
      .sort((a, b) => {
        if (a.id === 'other') return 1;
        if (b.id === 'other') return -1;
        return b.boardCount - a.boardCount;
      });

    return result;
  }, [boards, search]);

  // Preload all vendor logos
  const [logosLoaded, setLogosLoaded] = useState(false);
  useEffect(() => {
    if (!manufacturers.length || logosLoaded) return;

    const vendorsWithLogos = manufacturers.filter(m => vendorHasLogo(m.id) && m.id !== 'other');
    if (vendorsWithLogos.length === 0) {
      setLogosLoaded(true);
      return;
    }

    let loaded = 0;
    vendorsWithLogos.forEach(m => {
      const img = new Image();
      img.onload = img.onerror = () => {
        loaded++;
        if (loaded >= vendorsWithLogos.length) {
          setLogosLoaded(true);
        }
      };
      img.src = getVendorLogoUrl(m.id);
    });
  }, [manufacturers, logosLoaded]);

  const searchBarContent = (
    <div className="modal-search">
      <div className="search-box" style={{ marginBottom: 0 }}>
        <Search className="search-icon" size={18} />
        <input
          type="text"
          placeholder={t('modal.searchManufacturer')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="search-input"
          autoFocus
        />
      </div>
    </div>
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={t('modal.selectManufacturer')} searchBar={searchBarContent}>
      {loading || !logosLoaded ? (
        <div className="loading">
          <div className="spinner" />
          <p>{t('modal.loading')}</p>
        </div>
      ) : error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : (
        <div className="modal-list">
          {manufacturers.map((mfr) => {
            const config = MANUFACTURERS[mfr.id];
            return (
              <button
                key={mfr.id}
                className="list-item"
                onClick={() => onSelect(mfr)}
              >
                <ManufacturerIcon id={mfr.id} config={config} />
                <div className="list-item-content">
                  <div className="list-item-title">{mfr.name}</div>
                  <div className="list-item-subtitle">{mfr.boardCount} {t('home.boards')}</div>
                </div>
              </button>
            );
          })}
          {manufacturers.length === 0 && (
            <div className="no-results">
              <p>{t('modal.noManufacturers')}</p>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}

// Re-export from config for backward compatibility
// eslint-disable-next-line react-refresh/only-export-components
export { MANUFACTURERS, getManufacturer } from '../config';
