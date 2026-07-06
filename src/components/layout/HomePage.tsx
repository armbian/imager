import { useState, useEffect, type ReactNode, type ComponentType } from 'react';
import { Factory, Cpu, Database, HardDrive, Usb, FolderOpen, Archive, Check, ArrowRight, Lock, WifiOff } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getCachedBoardImage } from '../../hooks/useTauri';
import { IMAGE_VARIANT } from '../../config';
import { isDetectedBoard, formatImageIdentity } from '../../utils';
import type { BoardInfo, ImageInfo, BlockDevice, Manufacturer } from '../../types';
import { deriveFlashMethod, isEdlImage } from '../../types';
import { MarqueeText, MotdTip, BoardImage, UpdateEntry } from '../shared';
import { ManufacturerPanel } from './ManufacturerPanel';
import { BoardPanel } from './BoardPanel';
import { OsPanel } from './OsPanel';
import { DevicePanel } from './DevicePanel';

interface HomePageProps {
  selectedManufacturer: Manufacturer | null;
  selectedBoard: BoardInfo | null;
  selectedImage: ImageInfo | null;
  selectedDevice: BlockDevice | null;
  onChooseManufacturer: () => void;
  onChooseBoard: () => void;
  onChooseImage: () => void;
  onChooseDevice: () => void;
  onChooseCustomImage: () => void;
  onOpenCacheManager: () => void;
  /** Select a manufacturer directly from the inline browser. */
  onSelectManufacturer: (manufacturer: Manufacturer) => void;
  /** Select a board directly from the inline browser. */
  onSelectBoard: (board: BoardInfo) => void;
  /** Select an OS image directly from the inline browser. */
  onSelectImage: (image: ImageInfo) => void;
  /** Pick a storage device (populates the steps, shows the confirm summary). */
  onSelectDevice: (device: BlockDevice) => void;
  /** Confirm the inline storage summary and start flashing. */
  onConfirmDevice: () => void;
  /** Cancel the inline storage summary (back to the device list). */
  onClearDevice: () => void;
  isOnline?: boolean;
  /** One-shot entrance animation flag while leaving the welcome screen. */
  entering?: boolean;
}

/** Visual/interaction state of a step. */
type StepState = 'done' | 'active' | 'locked' | 'readonly';

/** Descriptor for a single step, shared by the sidebar nav and the focus panel. */
interface Step {
  key: string;
  index: number;
  icon: ComponentType<{ size?: number }>;
  label: string;
  cta: string;
  /** Selected value (already-localized node), absent when nothing chosen yet. */
  value?: ReactNode;
  /** Secondary muted line (size, image count, kernel branch). */
  meta?: ReactNode;
  state: StepState;
  /** The final write action, rendered with accent styling. */
  primary?: boolean;
  onClick?: () => void;
  /** Inline selector shown full-panel when this step is active (e.g. manufacturer grid). */
  panel?: ReactNode;
}

/** Sidebar navigation item. */
function SideStep({ step }: { step: Step }) {
  const { index, icon: Icon, label, value, state, onClick } = step;
  const isDone = state === 'done' || state === 'readonly';
  const clickable = (state === 'active' || state === 'done') && !!onClick;

  return (
    <button
      type="button"
      className={`side-step is-${state}`}
      onClick={clickable ? onClick : undefined}
      disabled={!clickable}
      aria-current={state === 'active'}
    >
      <span className="side-step__badge">{isDone ? <Check size={13} strokeWidth={3} /> : index}</span>
      <span className="side-step__icon"><Icon size={18} /></span>
      <span className="side-step__text">
        <span className="side-step__label">{label}</span>
        {value ? (
          <span className="side-step__value">{value}</span>
        ) : (
          <span className="side-step__value is-empty">{step.cta}</span>
        )}
      </span>
      {state === 'locked' && <Lock size={14} className="side-step__lock" />}
    </button>
  );
}

/** Branded OS identity string (e.g. "Armbian 26.5.1 GNOME"); shared with the flash header. */
function osLabelText(image: ImageInfo, t: (key: string) => string): string {
  return formatImageIdentity(image, t).title;
}

/** OS title/meta builders, reused across branches. */
function osTitle(image: ImageInfo, t: (key: string) => string): ReactNode {
  return <MarqueeText text={osLabelText(image, t)} className="val-text" />;
}
function osMeta(image: ImageInfo, t: (key: string) => string): ReactNode {
  return formatImageIdentity(image, t).meta;
}

/** Last step is a USB "device" (icon + wording) for EDL/QDL images, a "storage" drive otherwise. */
function deviceStepMeta(
  image: ImageInfo | null,
  t: (key: string) => string
): { icon: ComponentType<{ size?: number }>; label: string; cta: string } {
  return image && isEdlImage(image)
    ? { icon: Usb, label: t('home.device'), cta: t('home.chooseDevice') }
    : { icon: HardDrive, label: t('home.storage'), cta: t('home.chooseStorage') };
}

/** Step sidebar + focus panel that highlights the active step and previews the board. */
function SplitLayout({
  steps,
  boardImage,
  footer,
  fullBleed = false,
  entering = false,
  showMotd = true,
}: {
  steps: Step[];
  boardImage: string | null;
  footer?: ReactNode;
  /** Collapse the sidebar and expand the panel full-width (e.g. confirm view). */
  fullBleed?: boolean;
  /** One-shot entrance animation flag while leaving the welcome screen. */
  entering?: boolean;
  /** Hide the rotating MOTD tip (e.g. while offline, where it's irrelevant). */
  showMotd?: boolean;
}) {
  // The active step drives the focus panel; fall back to the last step when done.
  const active = steps.find((s) => s.state === 'active') ?? steps[steps.length - 1];
  const ActiveIcon = active.icon;
  const completed = steps.filter((s) => s.state === 'done' || s.state === 'readonly');

  return (
    <div className="home-page home-split">
      <div className={`split${fullBleed ? ' is-full' : ''}${entering ? ' is-entering' : ''}`}>
        <aside className="split-nav">
          <nav className="split-nav__list">
            {steps.map((step) => (
              <SideStep key={step.key} step={step} />
            ))}
          </nav>
          {/* Custom-image link + rotating tip, pinned to the bottom together */}
          <div className="split-nav__bottom">
            {footer && <div className="split-nav__foot">{footer}</div>}
            {showMotd && <MotdTip />}
            <UpdateEntry />
          </div>
        </aside>

        <section className={`split-main${active.panel ? ' is-panel' : ''}`}>
          {active.panel ? (
            active.panel
          ) : (
          <div className="focus">
            <div className="focus-visual">
              {boardImage ? (
                // BoardImage owns the logo watermark fallback when a photo fails.
                <BoardImage src={boardImage} alt="" />
              ) : (
                <span className="focus-visual__icon"><ActiveIcon size={44} /></span>
              )}
            </div>

            <span className="focus-kicker">{active.label}</span>
            <h2 className="focus-title">{active.value ?? active.cta}</h2>

            <button
              type="button"
              className={`focus-cta${active.primary ? ' is-primary' : ''}`}
              onClick={active.onClick}
              disabled={active.state === 'locked' || active.state === 'readonly'}
            >
              {active.cta}
              <ArrowRight size={17} />
            </button>

            {completed.length > 0 && (
              <ul className="focus-summary">
                {completed.map((s) => (
                  <li key={s.key}>
                    <Check size={13} strokeWidth={3} />
                    <span className="focus-summary__label">{s.label}</span>
                    <span className="focus-summary__value">{s.value}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
          )}
        </section>
      </div>
    </div>
  );
}

/** Main selection page: step sidebar + focus panel, with offline/custom/detected variants. */
export function HomePage({
  selectedManufacturer,
  selectedBoard,
  selectedImage,
  selectedDevice,
  onChooseManufacturer,
  onChooseBoard,
  onChooseImage,
  onChooseDevice,
  onChooseCustomImage,
  onOpenCacheManager,
  onSelectManufacturer,
  onSelectBoard,
  onSelectImage,
  onSelectDevice,
  onConfirmDevice,
  onClearDevice,
  isOnline = true,
  entering = false,
}: HomePageProps) {
  const { t } = useTranslation();
  const isCustomImage = selectedImage?.is_custom;
  const hasDetectedBoard = isDetectedBoard(selectedBoard);
  const isGenericCustom = isCustomImage && !hasDetectedBoard;
  const showOfflineLayout = !isOnline && !selectedManufacturer;

  // Preview the selected board's photo (real API slug only), with fallback.
  const previewSlug = hasDetectedBoard ? selectedBoard?.slug ?? null : null;
  const [boardImage, setBoardImage] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    if (!previewSlug) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- clear preview when no real board slug
      setBoardImage(null);
      return;
    }
    getCachedBoardImage(previewSlug)
      .then((uri) => alive && setBoardImage(uri))
      .catch(() => alive && setBoardImage(null));
    return () => {
      alive = false;
    };
  }, [previewSlug]);

  const deviceValue = selectedDevice ? (
    <MarqueeText text={selectedDevice.model || selectedDevice.name} className="val-text" />
  ) : undefined;
  const deviceMeta = selectedDevice ? selectedDevice.size_formatted : null;
  const dStep = deviceStepMeta(selectedImage, t);

  // Upstream selections shown in the inline storage confirm summary.
  const deviceSummary = [
    selectedManufacturer && { label: t('home.manufacturer'), value: selectedManufacturer.name },
    selectedBoard && { label: t('home.board'), value: selectedBoard.name },
    selectedImage && { label: t('home.operatingSystem'), value: osLabelText(selectedImage, t) },
  ].filter(Boolean) as { label: string; value: string }[];

  // Inline storage panel; it picks list vs. confirm view from `selectedDevice`.
  const renderDevicePanel = () => (
    <DevicePanel
      flashMethod={selectedImage ? deriveFlashMethod(selectedImage) : undefined}
      edlEntry={selectedBoard?.qdl?.edl_entry ?? null}
      summary={deviceSummary}
      boardImage={boardImage}
      selectedDevice={selectedDevice}
      onSelect={onSelectDevice}
      onConfirm={onConfirmDevice}
      onCancel={onClearDevice}
      supportsAutoconfig={!isGenericCustom}
    />
  );

  // Sidebar footer: the optional custom/change-image link (settings now lives in the header).
  const foot = (link?: ReactNode) => (link ? <div className="split-nav__actions">{link}</div> : null);

  // Offline entry point: pick a local source (no split, focused choice).
  if (showOfflineLayout) {
    return (
      <div className="home-page">
        {/* Two-column like the flash stage: animated hero left, choices right. */}
        <div className="offline">
          {/* Big animated glyph: ripple rings + warm glow + gentle float. */}
          <div className="offline__hero" aria-hidden="true">
            <span className="offline__ring" />
            <span className="offline__ring" />
            <WifiOff className="offline__glyph" size={72} strokeWidth={1.5} />
          </div>

          <div className="offline__main">
            <h2 className="offline__title">{t('home.offlineTitle')}</h2>
            <p className="offline__hint">{t('home.offlineHint')}</p>

            {/* Two quiet rows on one translucent surface, hairline between. */}
            <div className="offline__actions">
              <button type="button" className="offline__row" onClick={onChooseCustomImage}>
                <span className="offline__chip"><FolderOpen size={18} /></span>
                <span className="offline__label">{t('home.useCustomImage')}</span>
                <ArrowRight className="offline__arrow" size={16} />
              </button>
              <button type="button" className="offline__row" onClick={onOpenCacheManager}>
                <span className="offline__chip"><Archive size={18} /></span>
                <span className="offline__label">{t('home.cachedImages')}</span>
                <ArrowRight className="offline__arrow" size={16} />
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Generic custom image (no detected board): OS (fixed) + Storage.
  if (isGenericCustom && selectedImage) {
    const steps: Step[] = [
      {
        key: 'image',
        index: 1,
        icon: Database,
        label: t('home.operatingSystem'),
        cta: t('home.operatingSystem'),
        value: osTitle(selectedImage, t),
        meta: osMeta(selectedImage, t),
        state: 'readonly',
      },
      {
        key: 'device',
        index: 2,
        icon: dStep.icon,
        label: dStep.label,
        cta: dStep.cta,
        value: deviceValue,
        meta: deviceMeta,
        state: selectedDevice ? 'done' : 'active',
        primary: !selectedDevice,
        onClick: onChooseDevice,
        panel: renderDevicePanel(),
      },
    ];
    return (
      <SplitLayout
        steps={steps}
        boardImage={boardImage}
        fullBleed={!!selectedDevice}
        entering={entering}
        showMotd={isOnline}
        footer={foot(
          <button type="button" className="split-link" onClick={onChooseCustomImage}>
            <FolderOpen size={15} />
            {t('home.changeCustomImage')}
          </button>
        )}
      />
    );
  }

  // Custom/cached Armbian image with detected board: read-only rows + change image.
  if (isCustomImage && hasDetectedBoard && selectedImage && selectedBoard) {
    const isCached = selectedImage.image_variant === IMAGE_VARIANT.CACHED;
    const steps: Step[] = [
      {
        key: 'manufacturer',
        index: 1,
        icon: Factory,
        label: t('home.manufacturer'),
        cta: t('home.manufacturer'),
        value: <MarqueeText text={selectedManufacturer?.name || ''} className="val-text" />,
        state: 'readonly',
      },
      {
        key: 'board',
        index: 2,
        icon: Cpu,
        label: t('home.board'),
        cta: t('home.board'),
        value: <MarqueeText text={selectedBoard.name} className="val-text" />,
        meta: t('home.imageCount', { count: selectedBoard.image_count }),
        state: 'readonly',
      },
      {
        key: 'image',
        index: 3,
        icon: Database,
        label: t('home.operatingSystem'),
        cta: t('home.operatingSystem'),
        value: osTitle(selectedImage, t),
        meta: osMeta(selectedImage, t),
        state: 'readonly',
      },
      {
        key: 'device',
        index: 4,
        icon: dStep.icon,
        label: dStep.label,
        cta: dStep.cta,
        value: deviceValue,
        meta: deviceMeta,
        state: selectedDevice ? 'done' : 'active',
        primary: !selectedDevice,
        onClick: onChooseDevice,
        panel: renderDevicePanel(),
      },
    ];
    return (
      <SplitLayout
        steps={steps}
        boardImage={boardImage}
        fullBleed={!!selectedDevice}
        entering={entering}
        showMotd={isOnline}
        footer={foot(
          <button
            type="button"
            className="split-link"
            onClick={isCached ? onOpenCacheManager : onChooseCustomImage}
          >
            <FolderOpen size={15} />
            {isCached ? t('home.changeCachedImage') : t('home.changeCustomImage')}
          </button>
        )}
      />
    );
  }

  // Normal flow: 4 interactive steps with cascade locking.
  const steps: Step[] = [
    {
      key: 'manufacturer',
      index: 1,
      icon: Factory,
      label: t('home.manufacturer'),
      cta: t('home.chooseBrand'),
      value: selectedManufacturer ? <MarqueeText text={selectedManufacturer.name} className="val-text" /> : undefined,
      state: selectedManufacturer ? 'done' : 'active',
      onClick: onChooseManufacturer,
      panel: selectedManufacturer ? undefined : <ManufacturerPanel onSelect={onSelectManufacturer} />,
    },
    {
      key: 'board',
      index: 2,
      icon: Cpu,
      label: t('home.board'),
      cta: t('home.chooseBoard'),
      value: selectedBoard ? <MarqueeText text={selectedBoard.name} className="val-text" /> : undefined,
      meta: selectedBoard ? t('home.imageCount', { count: selectedBoard.image_count }) : null,
      state: !selectedManufacturer ? 'locked' : selectedBoard ? 'done' : 'active',
      onClick: onChooseBoard,
      panel: selectedManufacturer && !selectedBoard ? <BoardPanel manufacturer={selectedManufacturer} onSelect={onSelectBoard} /> : undefined,
    },
    {
      key: 'image',
      index: 3,
      icon: Database,
      label: t('home.operatingSystem'),
      cta: t('home.chooseOs'),
      value: selectedImage ? osTitle(selectedImage, t) : undefined,
      meta: selectedImage ? osMeta(selectedImage, t) : null,
      state: !selectedBoard ? 'locked' : selectedImage ? 'done' : 'active',
      onClick: onChooseImage,
      panel: selectedBoard && !selectedImage ? <OsPanel board={selectedBoard} onSelect={onSelectImage} /> : undefined,
    },
    {
      key: 'device',
      index: 4,
      icon: dStep.icon,
      label: dStep.label,
      cta: dStep.cta,
      value: deviceValue,
      meta: deviceMeta,
      state: !selectedImage ? 'locked' : selectedDevice ? 'done' : 'active',
      primary: !!selectedImage && !selectedDevice,
      onClick: onChooseDevice,
      panel: selectedImage ? renderDevicePanel() : undefined,
    },
  ];

  return (
    <SplitLayout
      steps={steps}
      boardImage={boardImage}
      fullBleed={!!selectedDevice}
      entering={entering}
      showMotd={isOnline}
      footer={foot(
        !selectedManufacturer ? (
          <button type="button" className="split-link" onClick={onChooseCustomImage}>
            <FolderOpen size={15} />
            {t('home.useCustomImage')}
          </button>
        ) : undefined
      )}
    />
  );
}
