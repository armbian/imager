/**
 * Cache Manager Modal
 *
 * Displays cached images organized by board with per-image
 * delete and "reuse for flashing" capabilities.
 * Uses the same visual patterns as ImageModal for consistency.
 */

import { useState, useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { X, ChevronRight, HardDrive, Trash2, Package, Monitor, Terminal, Zap, RotateCcw } from 'lucide-react';
import { listCachedImages, deleteCachedImage, getBoards, getBoardImageUrl, logWarn } from '../../hooks/useTauri';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { BoardBadges } from '../shared/BoardBadges';
import { useToasts } from '../../hooks/useToasts';
import { formatBytes, preloadImage, parseArmbianFilename } from '../../utils';
import { EVENTS } from '../../config';
import { getOsInfo } from '../../config/os-info';
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

/** Default background for unknown OS icons */
const DEFAULT_COLOR = 'var(--bg-secondary)';

/** Fallback board image (Armbian logo) */
const FALLBACK_IMAGE = '/armbian-logo_nofound.png';

/**
 * Format a relative time string from a Unix timestamp
 */
function formatRelativeTime(timestamp: number, t: (key: string, opts?: Record<string, unknown>) => string): string {
  const now = Math.floor(Date.now() / 1000);
  const diff = now - timestamp;

  if (diff < 60) return t('settings.cache.justNow');
  if (diff < 3600) return t('settings.cache.minutesAgo', { count: Math.floor(diff / 60) });
  if (diff < 86400) return t('settings.cache.hoursAgo', { count: Math.floor(diff / 3600) });
  return t('settings.cache.daysAgo', { count: Math.floor(diff / 86400) });
}

export function CacheManagerModal({ isOpen, onClose }: CacheManagerModalProps) {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const [cachedImages, setCachedImages] = useState<CachedImageInfo[]>([]);
  const [allBoards, setAllBoards] = useState<BoardInfo[]>([]);
  const [boardImageUrls, setBoardImageUrls] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());
  const [deleteTarget, setDeleteTarget] = useState<CachedImageInfo | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const { isExiting, handleClose } = useModalExitAnimation({ onClose });

  /** Close modal on Escape key */
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
        const [images, boards] = await Promise.all([
          listCachedImages(),
          getBoards(),
        ]);
        setCachedImages(images);
        setAllBoards(boards);

        // Preload board images in parallel
        const slugs = new Set(
          images.map((img) => img.board_slug).filter(Boolean) as string[]
        );
        const results = await Promise.all(
          Array.from(slugs).map(async (slug) => {
            try {
              const url = await getBoardImageUrl(slug);
              if (url) {
                await preloadImage(url);
                return { slug, url };
              }
            } catch { /* fallback */ }
            return null;
          })
        );
        const urls: Record<string, string> = {};
        for (const r of results) {
          if (r) urls[r.slug] = r.url;
        }
        setBoardImageUrls(urls);

        // Auto-expand all groups
        const allSlugs = new Set(images.map((img) => img.board_slug ?? '__unknown__'));
        setExpandedGroups(allSlugs);
      } catch (err) {
        logWarn('cache-manager', `Failed to load cache data: ${err}`);
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, [isOpen]);

  /** Group cached images by board slug */
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

  /** Toggle accordion group */
  const toggleGroup = useCallback((key: string) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  /** Handle delete confirmation */
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

  /** Handle reuse: dispatch event and close */
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

  const groupKey = (group: BoardGroup) => group.slug ?? '__unknown__';

  return (
    <>
      <div className={`modal-overlay ${isExiting ? 'modal-exiting' : 'modal-entering'}`} onClick={handleClose}>
        <div
          className={`modal modal-content cache-manager-modal ${isExiting ? 'modal-exiting' : 'modal-entering'}`}
          role="dialog"
          aria-modal="true"
          aria-labelledby="cache-manager-title"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="modal-header">
            <h2 id="cache-manager-title">{t('settings.cache.managerTitle')}</h2>
            <button className="modal-close" onClick={handleClose} aria-label="Close">
              <X size={20} />
            </button>
          </div>

          <div className="modal-body">
            {loading ? (
              <div className="cache-loading">{t('modal.loading')}</div>
            ) : cachedImages.length === 0 ? (
              <div className="cache-empty-state">
                <HardDrive size={48} />
                <p>{t('settings.cache.noCachedImages')}</p>
              </div>
            ) : (
              boardGroups.map((group) => {
                const key = groupKey(group);
                const isExpanded = expandedGroups.has(key);

                return (
                  <div key={key} className="cache-board-group">
                    {/* Board group header */}
                    <div
                      className="cache-group-header"
                      onClick={() => toggleGroup(key)}
                      role="button"
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          toggleGroup(key);
                        }
                      }}
                    >
                      <ChevronRight
                        size={16}
                        className={`cache-group-chevron ${isExpanded ? 'expanded' : ''}`}
                      />
                      <img
                        className="cache-group-thumb"
                        src={group.imageUrl ?? FALLBACK_IMAGE}
                        alt={group.name}
                        onError={(e) => {
                          (e.target as HTMLImageElement).src = FALLBACK_IMAGE;
                        }}
                      />
                      <div className="cache-group-info">
                        <div className="cache-group-title">
                          <span>{group.name}</span>
                          {group.board && (
                            <BoardBadges board={group.board} className="cache-inline-badges" />
                          )}
                        </div>
                        <div className="cache-group-meta">
                          {t('settings.cache.imageCount', { count: group.images.length })}
                          {' · '}
                          {formatBytes(group.totalSize)}
                        </div>
                      </div>
                    </div>

                    {/* Expanded image list — uses same pattern as ImageModal */}
                    {isExpanded && (
                      <div className="cache-group-content">
                        <div className="modal-list">
                          {group.images.map((image, index) => {
                            const parsed = parseArmbianFilename(image.filename);
                            const osInfo = parsed?.distro ? getOsInfo(parsed.distro) : null;
                            const desktopEnv = parsed?.desktop ? getDesktopEnv(parsed.desktop) : null;
                            const kernelType = parsed?.branch ? getKernelType(parsed.branch) : null;
                            const badgeConfig = kernelType ? KERNEL_BADGES[kernelType] : null;

                            return (
                              <div
                                key={image.path}
                                className="list-item cache-list-item"
                                style={{ animationDelay: `${index * 25}ms` }}
                              >
                                {/* OS Icon — same as ImageModal */}
                                <div className="list-item-icon os-icon" style={{ backgroundColor: osInfo?.color || DEFAULT_COLOR }}>
                                  {osInfo?.logo ? (
                                    <img src={osInfo.logo} alt={osInfo.name} />
                                  ) : (
                                    <Package size={32} color="white" />
                                  )}
                                </div>

                                {/* Content — same structure as ImageModal */}
                                <div className="list-item-content">
                                  <div className="list-item-title">
                                    {parsed?.version
                                      ? `Armbian ${parsed.version}`
                                      : image.filename}
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
                                        {parsed?.kernel && (
                                          <span style={{ opacity: 0.8, marginLeft: 1 }}>{parsed.kernel}</span>
                                        )}
                                      </div>
                                    )}
                                  </div>

                                  <div className="list-item-meta">
                                    {formatBytes(image.size)} · {formatRelativeTime(image.last_used, t)}
                                  </div>
                                </div>

                                {/* Right side: action buttons */}
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
                      </div>
                    )}
                  </div>
                );
              })
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
    </>
  );
}
