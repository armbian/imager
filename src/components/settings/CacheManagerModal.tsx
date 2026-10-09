// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState, useEffect, useMemo, useCallback, useRef, type CSSProperties, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { X, Archive, Trash2, RotateCcw, Package, RefreshCw, Loader2 } from 'lucide-react';
import { listCachedImages, deleteCachedImage, getBoards, getCachedBoardImage, logWarn } from '../../hooks/useTauri';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { ErrorDisplay } from '../shared/ErrorDisplay';
import { BoardBadges } from '../shared/BoardBadges';
import { BoardImage } from '../shared/BoardImage';
import { MarqueeText } from '../shared/MarqueeText';
import { useToasts } from '../../hooks/useToasts';
import { formatBytes, parseArmbianFilename, formatRelativeTime, splitArmbianVersion, getErrorMessage, splitUfsKernel, hexToRgba } from '../../utils';
import { EVENTS, UI, COLORS } from '../../config';
import { getOsInfo } from '../../config/os-info';
import { getMonoLogo } from '../../config/mono-logos';
import { distroBlock } from '../../utils/distroTheme';
import { getVariantBadge, getKernelType, KERNEL_BADGES, STORAGE_BADGES, CLI_BADGE } from '../../config/badges';
import { IMAGE_STORAGE, type CachedImageInfo, type BoardInfo } from '../../types';

const UNKNOWN_BOARD_GROUP = '__unknown__';

interface CacheManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
  /** WebKit does not focus a clicked button, so the opener is named instead of read from activeElement */
  returnFocusRef?: RefObject<HTMLElement | null>;
}

interface BoardGroup {
  key: string;
  name: string;
  board: BoardInfo | null;
  imageUrl: string | null;
  images: CachedImageInfo[];
  totalSize: number;
}

function tagVars(hex: string): CSSProperties {
  return { '--tag': hex, '--tag-soft': hexToRgba(hex, UI.TAG_ALPHA.SOFT), '--tag-ring': hexToRgba(hex, UI.TAG_ALPHA.RING) } as CSSProperties;
}

function SoftTag({ label, color }: { label: string; color: string }) {
  return (
    <span className="cache-modal__tag" style={tagVars(color)}>
      <i aria-hidden="true" />
      {label}
    </span>
  );
}

function CachedImageRow({ image, index, busy, onUse, onDelete }: {
  image: CachedImageInfo;
  index: number;
  busy: boolean;
  onUse: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();
  const parsed = parseArmbianFilename(image.filename);
  const osInfo = parsed?.distro ? getOsInfo(parsed.distro) : null;
  const osName = osInfo?.name || parsed?.distro || '';
  const monoLogo = getMonoLogo(parsed?.distro ?? '', parsed?.desktop);
  const variant = getVariantBadge(parsed?.desktop) ?? CLI_BADGE;
  const kernelType = parsed?.branch ? getKernelType(parsed.branch) : null;
  const kernel = kernelType ? KERNEL_BADGES[kernelType] : null;
  const { kernel: kernelVersion, isUfs } = splitUfsKernel(parsed?.kernel ?? null);
  const { base: baseVersion, build } = splitArmbianVersion(parsed?.version ?? '');
  const title = parsed?.version ? `Armbian ${baseVersion}` : image.filename;
  const meta = [osInfo?.name, build, formatBytes(image.size), formatRelativeTime(image.last_used, t)].filter(Boolean).join(UI.SUMMARY_SEPARATOR);

  return (
    <li className="cache-modal__image" style={{ animationDelay: `${index * UI.CACHE_ROW_STAGGER_MS}ms` }}>
      <span className="cache-modal__os" style={{ background: distroBlock(osName) }}>
        {monoLogo ? (
          <img src={monoLogo} alt={osName} />
        ) : (
          <Package size={22} color={COLORS.ON_TILE} aria-hidden="true" />
        )}
      </span>
      <span className="cache-modal__image-text">
        <span className="cache-modal__image-line">
          <b>{title}</b>
          <SoftTag label={variant.label} color={variant.color} />
          {kernel && <SoftTag label={kernelVersion ? `${kernel.label} ${kernelVersion}` : kernel.label} color={kernel.color} />}
          {isUfs && <SoftTag label={STORAGE_BADGES[IMAGE_STORAGE.UFS].label} color={STORAGE_BADGES[IMAGE_STORAGE.UFS].color} />}
        </span>
        <small>{meta}</small>
      </span>
      <button type="button" className="btn btn-secondary btn-pill" onClick={onUse}>
        <RotateCcw size={16} aria-hidden="true" />
        {t('settings.cache.useImage')}
      </button>
      <button
        type="button"
        className="cache-modal__delete"
        onClick={onDelete}
        disabled={busy}
        aria-label={`${t('settings.cache.deleteImage')}: ${title}${osName ? ` ${osName}` : ''}`}
      >
        <Trash2 size={16} aria-hidden="true" />
      </button>
    </li>
  );
}

function CacheManagerDialog({ onClose, returnFocusRef }: Omit<CacheManagerModalProps, 'isOpen'>) {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();
  const closeRef = useRef<HTMLButtonElement>(null);

  const [cachedImages, setCachedImages] = useState<CachedImageInfo[]>([]);
  const [allBoards, setAllBoards] = useState<BoardInfo[]>([]);
  const [boardImageUrls, setBoardImageUrls] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<CachedImageInfo | null>(null);
  const [deleteAllGroup, setDeleteAllGroup] = useState<BoardGroup | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const { isExiting, handleClose } = useModalExitAnimation({ onClose });
  const confirmOpen = deleteTarget !== null || deleteAllGroup !== null;

  useEffect(() => {
    const opener = returnFocusRef?.current ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    closeRef.current?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, [returnFocusRef]);

  useEffect(() => {
    // The confirmation dialog has no Escape handling of its own, so Escape cancels it first.
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented) return;
      e.preventDefault();
      if (confirmOpen) {
        if (!isDeleting) {
          setDeleteTarget(null);
          setDeleteAllGroup(null);
        }
        return;
      }
      handleClose();
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [confirmOpen, isDeleting, handleClose]);

  const loadData = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const images = await listCachedImages();
      setCachedImages(images);

      // Board data only adds tier badges and full names; offline falls back to filename metadata.
      try {
        setAllBoards(await getBoards());
      } catch {
        setAllBoards([]);
      }

      const slugs = Array.from(new Set(images.map((img) => img.board_slug).filter((s): s is string => Boolean(s))));
      const results = await Promise.all(
        slugs.map(async (slug) => {
          try {
            const url = await getCachedBoardImage(slug);
            return url ? ([slug, url] as const) : null;
          } catch {
            return null;
          }
        })
      );
      setBoardImageUrls(Object.fromEntries(results.filter((r) => r !== null)));
      setSelectedKey(null);
    } catch (err) {
      logWarn('cache-manager', `Failed to load cache data: ${err}`);
      setLoadError(getErrorMessage(err, t('settings.cache.loadError')));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const boardGroups = useMemo((): BoardGroup[] => {
    const groupMap = new Map<string, CachedImageInfo[]>();
    for (const img of cachedImages) {
      const key = img.board_slug ?? UNKNOWN_BOARD_GROUP;
      groupMap.set(key, [...(groupMap.get(key) ?? []), img]);
    }
    return Array.from(groupMap.entries()).map(([key, images]) => {
      const slug = key === UNKNOWN_BOARD_GROUP ? null : key;
      const board = slug ? allBoards.find((b) => b.slug === slug) ?? null : null;
      return {
        key,
        name: board?.name ?? images[0]?.board_name ?? t('settings.cache.unknownBoard'),
        board,
        imageUrl: slug ? boardImageUrls[slug] ?? null : null,
        images,
        totalSize: images.reduce((sum, img) => sum + img.size, 0),
      };
    });
  }, [cachedImages, allBoards, boardImageUrls, t]);

  // A deleted or never-chosen selection falls back to the first board.
  const selectedGroup = boardGroups.find((g) => g.key === selectedKey) ?? boardGroups[0] ?? null;

  const handleDeleteConfirm = async () => {
    if (!deleteTarget) return;
    setIsDeleting(true);
    try {
      await deleteCachedImage(deleteTarget.filename);
      setCachedImages((prev) => prev.filter((img) => img.filename !== deleteTarget.filename));
      showSuccess(t('settings.cache.deleteSuccess'));
    } catch {
      showError(t('settings.cache.deleteError'));
    } finally {
      setIsDeleting(false);
      setDeleteTarget(null);
    }
  };

  const handleDeleteAllConfirm = async () => {
    if (!deleteAllGroup) return;
    setIsDeleting(true);
    const filenames = new Set(deleteAllGroup.images.map((img) => img.filename));
    try {
      await Promise.all(deleteAllGroup.images.map((img) => deleteCachedImage(img.filename)));
      setCachedImages((prev) => prev.filter((img) => !filenames.has(img.filename)));
      showSuccess(t('settings.cache.deleteSuccess'));
    } catch {
      showError(t('settings.cache.deleteError'));
    } finally {
      setIsDeleting(false);
      setDeleteAllGroup(null);
    }
  };

  const handleReuse = (image: CachedImageInfo) => {
    window.dispatchEvent(
      new CustomEvent(EVENTS.CACHE_IMAGE_REUSE, {
        detail: {
          path: image.path,
          filename: image.filename,
          size: image.size,
          boardSlug: image.board_slug,
          boardName: image.board_name,
        },
      })
    );
    onClose();
  };

  const isEmpty = !loading && !loadError && cachedImages.length === 0;
  const summary = loading || loadError
    ? null
    : isEmpty
      ? t('settings.downloads.manageEmpty')
      : [
          t('settings.cache.imageCount', { count: cachedImages.length }),
          t('settings.cache.boardCount', { count: boardGroups.length }),
        ].join(UI.SUMMARY_SEPARATOR);

  const renderBody = () => {
    if (loading) {
      return (
        <div className="cache-modal__state" role="status">
          <Loader2 size={20} className="spinning" aria-hidden="true" />
          {t('modal.loading')}
        </div>
      );
    }
    if (loadError) {
      return (
        <div className="cache-modal__state">
          <ErrorDisplay error={loadError} onRetry={loadData} compact />
        </div>
      );
    }
    if (isEmpty || !selectedGroup) {
      return (
        <div className="cache-modal__state cache-modal__empty">
          <span className="cache-modal__empty-disc" aria-hidden="true">
            <Archive size={40} strokeWidth={1.5} />
          </span>
          <h3>{t('settings.cache.noCachedImages')}</h3>
          <p>{t('settings.cache.emptyHint')}</p>
          <button type="button" className="btn btn-secondary btn-pill" onClick={loadData}>
            <RefreshCw size={16} aria-hidden="true" />
            {t('device.refresh')}
          </button>
        </div>
      );
    }

    return (
      <div className="cache-modal__split">
        <div className="cache-modal__boards">
          {boardGroups.map((group) => {
            const active = group.key === selectedGroup.key;
            return (
              <button
                key={group.key}
                type="button"
                className={`cache-modal__board${active ? ' is-active' : ''}`}
                aria-pressed={active}
                onClick={() => setSelectedKey(group.key)}
              >
                <span className="cache-modal__board-text">
                  <MarqueeText text={group.name} className="cache-modal__board-name" />
                  <small>{t('settings.cache.imageCount', { count: group.images.length })}</small>
                </span>
                <span className="cache-modal__board-size">{formatBytes(group.totalSize)}</span>
              </button>
            );
          })}
        </div>

        <div className="cache-modal__detail">
          <div className="cache-modal__head">
            <span className="cache-modal__photo">
              <BoardImage className="cache-modal__photo-media" src={selectedGroup.imageUrl} alt={selectedGroup.name} />
            </span>
            <span className="cache-modal__head-text">
              <MarqueeText text={selectedGroup.name} className="cache-modal__head-name" />
              <span className="cache-modal__head-meta">
                {selectedGroup.board && <BoardBadges board={selectedGroup.board} className="cache-modal__tier" />}
                <span>{t('settings.cache.totalSize', { size: formatBytes(selectedGroup.totalSize) })}</span>
              </span>
            </span>
            <button
              type="button"
              className="btn btn-secondary btn-pill"
              onClick={() => setDeleteAllGroup(selectedGroup)}
              disabled={isDeleting}
            >
              <Trash2 size={16} aria-hidden="true" />
              {t('settings.cache.deleteAll')}
            </button>
          </div>

          <h4 className="cache-modal__count">
            {t('settings.cache.imageCount', { count: selectedGroup.images.length })}
          </h4>

          <ul className="cache-modal__images" key={selectedGroup.key}>
            {selectedGroup.images.map((image, index) => (
              <CachedImageRow
                key={image.path}
                image={image}
                index={index}
                busy={isDeleting}
                onUse={() => handleReuse(image)}
                onDelete={() => setDeleteTarget(image)}
              />
            ))}
          </ul>
        </div>
      </div>
    );
  };

  // Portal to <body> so the fixed overlay escapes the transformed settings page.
  return createPortal(
    <>
      <div className={`modal-overlay ${isExiting ? 'modal-exiting' : 'modal-entering'}`} onClick={handleClose}>
        <div
          className={`modal cache-modal ${isExiting ? 'modal-exiting' : 'modal-entering'}`}
          role="dialog"
          aria-modal="true"
          aria-labelledby="cache-manager-title"
          onClick={(e) => e.stopPropagation()}
        >
          <header className="cache-modal__header">
            <h2 id="cache-manager-title">{t('settings.cache.managerTitle')}</h2>
            {summary && <span className="cache-modal__summary">{summary}</span>}
            <button ref={closeRef} type="button" className="modal-close cache-modal__close" onClick={handleClose} aria-label={t('common.close')}>
              <X size={20} aria-hidden="true" />
            </button>
          </header>
          <div className="cache-modal__body">{renderBody()}</div>
        </div>
      </div>

      <ConfirmationDialog
        isOpen={deleteTarget !== null}
        title={t('settings.cache.deleteImage')}
        message={t('settings.cache.deleteConfirmSingle')}
        confirmText={t('settings.cache.deleteImage')}
        isDanger
        onCancel={() => setDeleteTarget(null)}
        onConfirm={handleDeleteConfirm}
      />

      <ConfirmationDialog
        isOpen={deleteAllGroup !== null}
        title={t('settings.cache.deleteAll')}
        message={t('settings.cache.deleteConfirmAll', { count: deleteAllGroup?.images.length ?? 0 })}
        confirmText={t('settings.cache.deleteAll')}
        isDanger
        onCancel={() => setDeleteAllGroup(null)}
        onConfirm={handleDeleteAllConfirm}
      />
    </>,
    document.body
  );
}

export function CacheManagerModal({ isOpen, onClose, returnFocusRef }: CacheManagerModalProps) {
  return isOpen ? <CacheManagerDialog onClose={onClose} returnFocusRef={returnFocusRef} /> : null;
}
