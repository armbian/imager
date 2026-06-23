// Cache manager sub-modal: master-detail (boards in left rail, selected board's cached images on right).
// Frozen `cache-*` vocabulary; glass restyle lives in settings.css.

import { useState, useEffect, useMemo, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { X, Archive, Trash2, Monitor, Terminal, Zap, RotateCcw, Package, HardDrive } from 'lucide-react';
import { listCachedImages, deleteCachedImage, getBoards, getCachedBoardImage, logWarn } from '../../hooks/useTauri';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { BoardBadges } from '../shared/BoardBadges';
import { BoardImage } from '../shared/BoardImage';
import { useToasts } from '../../hooks/useToasts';
import { formatBytes, parseArmbianFilename, formatRelativeTime, splitArmbianVersion } from '../../utils';
import { EVENTS } from '../../config';
import { getOsInfo } from '../../config/os-info';
import { getMonoLogo } from '../../config/mono-logos';
import { distroBlock } from '../../utils/distroTheme';
import { getDesktopEnv, getKernelType, DESKTOP_BADGES, KERNEL_BADGES, adjustBrightness } from '../../config/badges';
import type { CachedImageInfo, BoardInfo } from '../../types';

interface CacheManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/** Board group with matched board data and cached images */
interface BoardGroup {
  slug: string | null;
  name: string;
  board: BoardInfo | null;
  imageUrl: string | null;
  images: CachedImageInfo[];
  totalSize: number;
}

export function CacheManagerModal({ isOpen, onClose }: CacheManagerModalProps) {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const [cachedImages, setCachedImages] = useState<CachedImageInfo[]>([]);
  const [allBoards, setAllBoards] = useState<BoardInfo[]>([]);
  const [boardImageUrls, setBoardImageUrls] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  // Master-detail: which board group is shown in the right panel (null = derive first).
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<CachedImageInfo | null>(null);
  // When set, the confirm dialog deletes every cached image of this board group.
  const [deleteAllGroup, setDeleteAllGroup] = useState<BoardGroup | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const { isExiting, handleClose } = useModalExitAnimation({ onClose });

  useEffect(() => {
    if (!isOpen) return;
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') handleClose();
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [isOpen, handleClose]);

  /** Load cached images, board data, and preload thumbnails */
  useEffect(() => {
    if (!isOpen) return;
    setLoading(true);

    const loadData = async () => {
      try {
        // Load cached images first (always local, works offline)
        const images = await listCachedImages();
        setCachedImages(images);

        // Board data is optional, used for badges and full names.
        // When offline, we gracefully degrade to filename-based metadata.
        try {
          const boards = await getBoards();
          setAllBoards(boards);
        } catch {
          setAllBoards([]);
        }

        // Load cached board images (base64 data URIs) in parallel
        const slugs = new Set(
          images.map((img) => img.board_slug).filter(Boolean) as string[]
        );
        const results = await Promise.all(
          Array.from(slugs).map(async (slug) => {
            try {
              const dataUri = await getCachedBoardImage(slug);
              if (dataUri) return { slug, url: dataUri };
            } catch { /* fallback to default image */ }
            return null;
          })
        );
        const urls: Record<string, string> = {};
        for (const r of results) {
          if (r) urls[r.slug] = r.url;
        }
        setBoardImageUrls(urls);

        // Fresh view starts with no explicit selection (derived to first group).
        setSelectedKey(null);
      } catch (err) {
        logWarn('cache-manager', `Failed to load cache data: ${err}`);
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, [isOpen]);

  const boardGroups = useMemo((): BoardGroup[] => {
    const groupMap = new Map<string, CachedImageInfo[]>();

    for (const img of cachedImages) {
      const key = img.board_slug ?? '__unknown__';
      const existing = groupMap.get(key);
      if (existing) {
        existing.push(img);
      } else {
        groupMap.set(key, [img]);
      }
    }

    return Array.from(groupMap.entries()).map(([key, images]) => {
      const slug = key === '__unknown__' ? null : key;
      const matchedBoard = slug
        ? allBoards.find((b) => b.slug === slug) ?? null
        : null;
      const name = matchedBoard?.name ?? images[0]?.board_name ?? t('settings.cache.unknownBoard');

      return {
        slug,
        name,
        board: matchedBoard,
        imageUrl: slug ? (boardImageUrls[slug] ?? null) : null,
        images,
        totalSize: images.reduce((sum, img) => sum + img.size, 0),
      };
    });
  }, [cachedImages, allBoards, boardImageUrls, t]);

  const groupKey = useCallback((group: BoardGroup) => group.slug ?? '__unknown__', []);

  // Resolve the active group: honor selectedKey when still present, else fall back
  // to the first group (covers initial load and the case where it was just deleted).
  const selectedGroup = useMemo(() => {
    if (boardGroups.length === 0) return null;
    return (
      boardGroups.find((g) => groupKey(g) === selectedKey) ?? boardGroups[0]
    );
  }, [boardGroups, selectedKey, groupKey]);

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

  /** Delete every cached image belonging to a board group. */
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

  const handleReuse = useCallback(
    (image: CachedImageInfo) => {
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
    },
    [onClose]
  );

  if (!isOpen) return null;

  // Portal to <body> so the fixed overlay escapes the animated settings shell's
  // containing block (otherwise it would render trapped inside the panel).
  return createPortal(
    <>
      <div className={`modal-overlay ${isExiting ? 'modal-exiting' : 'modal-entering'}`} onClick={handleClose}>
        {/* Same glass shell as the Settings modal for a consistent premium look. */}
        <div
          className="settings-shell cache-shell"
          role="dialog"
          aria-modal="true"
          aria-labelledby="cache-manager-title"
          onClick={(e) => e.stopPropagation()}
        >
          <header className="settings-shell__header">
            <h2 className="settings-shell__title" id="cache-manager-title">{t('settings.cache.managerTitle')}</h2>
            <button className="modal-close" onClick={handleClose} aria-label="Close">
              <X size={20} />
            </button>
          </header>

          <div className="settings-shell__content cache-content">
            {loading ? (
              <div className="cache-loading">{t('modal.loading')}</div>
            ) : cachedImages.length === 0 ? (
              <div className="cache-empty-state">
                <span className="cache-empty-state__disc">
                  <Archive size={40} strokeWidth={1.5} />
                </span>
                <p>{t('settings.cache.noCachedImages')}</p>
              </div>
            ) : (
              <div className="cache-split">
                {/* LEFT RAIL — one board per row, click to select. */}
                <div className="cache-rail">
                  {boardGroups.map((group) => {
                    const key = groupKey(group);
                    const isActive = selectedGroup ? groupKey(selectedGroup) === key : false;

                    return (
                      <button
                        key={key}
                        className={`cache-rail-item ${isActive ? 'is-active' : ''}`}
                        onClick={() => setSelectedKey(key)}
                      >
                        {/* Text-only rail; the board photo lives once in the detail header. */}
                        <div className="cache-rail-info">
                          {/* Name + image count; size lives in the detail header. */}
                          <div className="cache-rail-title">{group.name}</div>
                          <div className="cache-rail-meta">
                            {t('settings.cache.imageCount', { count: group.images.length })}
                          </div>
                        </div>
                      </button>
                    );
                  })}
                </div>

                {/* RIGHT PANEL — every cached image of the selected board. */}
                {selectedGroup && (
                  <div className="cache-detail">
                    <div className="cache-detail-head">
                      <div className="cache-detail-thumb">
                        <BoardImage
                          className="cache-thumb-media"
                          src={selectedGroup.imageUrl}
                          alt={selectedGroup.name}
                        />
                      </div>
                      <div className="cache-detail-headinfo">
                        {/* Name and tier badge share one line. */}
                        <div className="cache-detail-titlerow">
                          <h3 className="cache-detail-name">{selectedGroup.name}</h3>
                          {selectedGroup.board && (
                            <BoardBadges board={selectedGroup.board} className="cache-inline-badges" />
                          )}
                        </div>
                        {/* Total size only; the per-board image count lives in the section divider. */}
                        <div className="cache-detail-sub">
                          {t('settings.cache.totalSize', { size: formatBytes(selectedGroup.totalSize) })}
                        </div>
                      </div>
                      <button
                        className="cache-detail-delall"
                        onClick={() => setDeleteAllGroup(selectedGroup)}
                        disabled={isDeleting}
                      >
                        {t('settings.cache.deleteAll')}
                      </button>
                    </div>

                    <div className="cache-detail-section">
                      {t('settings.cache.imageCount', { count: selectedGroup.images.length })}
                    </div>

                    {selectedGroup.images.map((image, index) => {
                      const parsed = parseArmbianFilename(image.filename);
                      const osInfo = parsed?.distro ? getOsInfo(parsed.distro) : null;
                      const monoLogo = getMonoLogo(parsed?.distro ?? '', parsed?.desktop);
                      const desktopEnv = parsed?.desktop ? getDesktopEnv(parsed.desktop) : null;
                      const kernelType = parsed?.branch ? getKernelType(parsed.branch) : null;
                      const badgeConfig = kernelType ? KERNEL_BADGES[kernelType] : null;
                      const isUfs = !!parsed?.kernel && parsed.kernel.toLowerCase().endsWith('-ufs');
                      const kernelVersion = isUfs ? parsed!.kernel!.slice(0, -4) : parsed?.kernel ?? null;
                      // Split "26.2.0-trunk.904" → base headline + build suffix (shown in meta).
                      const { base: baseVersion, build } = splitArmbianVersion(parsed?.version ?? '');

                      return (
                        <div
                          key={image.path}
                          className="cache-image-row"
                          style={{ animationDelay: `${index * 25}ms` }}
                        >
                          {/* Distro-tinted tile with a white mark anchors each row (matches the OS gallery). */}
                          <div
                            className="cache-image-os"
                            style={{ background: distroBlock(osInfo?.name || parsed?.distro || '') }}
                          >
                            {monoLogo ? (
                              <img
                                className="cache-image-os__logo"
                                src={monoLogo}
                                alt={osInfo?.name || parsed?.distro || ''}
                              />
                            ) : (
                              <Package size={28} color="#fff" />
                            )}
                          </div>

                          <div className="list-item-content">
                            <div className="cache-image-ver">
                              {parsed?.version ? `Armbian ${baseVersion}` : image.filename}
                            </div>

                            <div className="image-info-side-panel">
                              {desktopEnv && DESKTOP_BADGES[desktopEnv] ? (
                                <div
                                  className="side-info-badge"
                                  style={{
                                    background: 'linear-gradient(135deg, #3b82f6 0%, #2563eb 100%)',
                                    boxShadow: '0 2px 6px rgba(59, 130, 246, 0.4)',
                                    border: 'none',
                                    color: 'white',
                                  }}
                                >
                                  <Monitor size={11} />
                                  <span>{DESKTOP_BADGES[desktopEnv].label}</span>
                                </div>
                              ) : (
                                <div
                                  className="side-info-badge"
                                  style={{
                                    background: 'linear-gradient(135deg, #64748b 0%, #475569 100%)',
                                    boxShadow: '0 2px 6px rgba(100, 116, 139, 0.3)',
                                    border: 'none',
                                    color: 'white',
                                  }}
                                >
                                  <Terminal size={11} />
                                  <span>CLI</span>
                                </div>
                              )}
                              {badgeConfig && (
                                <div
                                  className="side-info-badge badge-kernel"
                                  style={{
                                    background: `linear-gradient(135deg, ${badgeConfig.color} 0%, ${adjustBrightness(badgeConfig.color, -20)} 100%)`,
                                    boxShadow: `0 2px 6px ${badgeConfig.color}66`,
                                    border: 'none',
                                    color: 'white',
                                  }}
                                >
                                  <Zap size={11} />
                                  <span>{badgeConfig.label}</span>
                                  {kernelVersion && (
                                    <span style={{ opacity: 0.8, marginLeft: 1 }}>{kernelVersion}</span>
                                  )}
                                </div>
                              )}
                              {isUfs && (
                                <div
                                  className="side-info-badge"
                                  style={{
                                    background: `linear-gradient(135deg, #f59e0b 0%, ${adjustBrightness('#f59e0b', -20)} 100%)`,
                                    boxShadow: '0 2px 6px #f59e0b66',
                                    border: 'none',
                                    color: 'white',
                                  }}
                                >
                                  <HardDrive size={11} />
                                  <span>UFS</span>
                                </div>
                              )}
                            </div>

                            {/* OS release (e.g. "Ubuntu 26.04") leads so stable rows are never
                                bare; build (trunk.NNN) is appended only when present. */}
                            <div className="list-item-meta">
                              {osInfo?.name && <>{osInfo.name} · </>}
                              {build && <>{build} · </>}
                              {formatBytes(image.size)} · {formatRelativeTime(image.last_used, t)}
                            </div>
                          </div>

                          <div className="cache-item-actions">
                            <button
                              className="cache-btn cache-btn-use"
                              onClick={() => handleReuse(image)}
                              title={t('settings.cache.useImage')}
                            >
                              <RotateCcw size={14} />
                              <span>{t('settings.cache.useImage')}</span>
                            </button>
                            <button
                              className="cache-btn cache-btn-delete"
                              onClick={() => setDeleteTarget(image)}
                              disabled={isDeleting}
                              title={t('settings.cache.deleteImage')}
                            >
                              <Trash2 size={14} />
                            </button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      <ConfirmationDialog
        isOpen={deleteTarget !== null}
        title={t('settings.cache.deleteImage')}
        message={t('settings.cache.deleteConfirmSingle')}
        confirmText={t('settings.cache.deleteImage')}
        isDanger={true}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={handleDeleteConfirm}
      />

      <ConfirmationDialog
        isOpen={deleteAllGroup !== null}
        title={t('settings.cache.deleteAll')}
        message={t('settings.cache.deleteConfirmAll', { count: deleteAllGroup?.images.length ?? 0 })}
        confirmText={t('settings.cache.deleteAll')}
        isDanger={true}
        onCancel={() => setDeleteAllGroup(null)}
        onConfirm={handleDeleteAllConfirm}
      />
    </>,
    document.body,
  );
}
