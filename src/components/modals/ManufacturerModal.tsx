import { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay, ListItemSkeleton, SearchBox } from '../shared';
import type { BoardInfo, Manufacturer } from '../../types';
import { getBoards } from '../../hooks/useTauri';
import { useAsyncDataWhen } from '../../hooks/useAsyncData';
import { useManufacturerList, type ManufacturerData } from '../../hooks/useVendorLogos';
import { DEFAULT_COLOR } from '../../utils';
import { UI, VENDOR } from '../../config';

// Re-export Manufacturer type for backward compatibility
export type { Manufacturer } from '../../types';

function ManufacturerIcon({ manufacturer }: { manufacturer: ManufacturerData }) {
  if (!manufacturer.logo || manufacturer.id === VENDOR.FALLBACK_ID) {
    return (
      <div className="list-item-icon" style={{ backgroundColor: DEFAULT_COLOR }}>
        {manufacturer.name.substring(0, 2).toUpperCase()}
      </div>
    );
  }

  return (
    <div className="list-item-icon manufacturer-logo">
      <img
        src={manufacturer.logo}
        alt={manufacturer.name}
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
  const [showSkeleton, setShowSkeleton] = useState(false);

  // Use hook for async data fetching
  const { data: boards, loading, error, reload } = useAsyncDataWhen<BoardInfo[]>(
    isOpen,
    () => getBoards(),
    [isOpen]
  );

  // Use shared hook for manufacturer list with logo validation
  const { manufacturers, isLoaded: logosLoaded } = useManufacturerList(boards, isOpen, search);

  // Derive ready state
  const manufacturersReady = useMemo(() => {
    return manufacturers && manufacturers.length > 0 && logosLoaded;
  }, [manufacturers, logosLoaded]);

  // Show skeleton with minimum delay
  useEffect(() => {
    let skeletonTimeout: NodeJS.Timeout;

    if (loading) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Show skeleton during loading
      setShowSkeleton(true);
    } else if (manufacturersReady) {
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
  }, [loading, manufacturersReady]);

  const searchBarContent = (
    <SearchBox
      value={search}
      onChange={setSearch}
      placeholder={t('modal.searchManufacturer')}
    />
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={t('modal.selectManufacturer')} searchBar={searchBarContent}>
      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : (
        <>
          {showSkeleton && <ListItemSkeleton count={UI.SKELETON.MANUFACTURER_MODAL} />}
          <div className="modal-list">
            {!showSkeleton && manufacturers.map((mfr) => (
              <button
                key={mfr.id}
                className="list-item"
                onClick={() => onSelect({ id: mfr.id, name: mfr.name, color: DEFAULT_COLOR, boardCount: mfr.boardCount })}
              >
                <ManufacturerIcon manufacturer={mfr} />
                <div className="list-item-content">
                  <div className="list-item-title">{mfr.name}</div>
                  <div className="list-item-subtitle">{t('home.boardCount', { count: mfr.boardCount })}</div>
                </div>
              </button>
            ))}
            {!showSkeleton && manufacturers.length === 0 && (
              <div className="no-results">
                <p>{t('modal.noManufacturers')}</p>
              </div>
            )}
          </div>
        </>
      )}
    </Modal>
  );
}
