import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import {
  RefreshCw, TriangleAlert, Shield, Usb, Lock, Cpu, ArrowRight, ChevronDown, Plus,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ErrorDisplay, DeviceIcon, getDeviceBadge, BoardImage, MarqueeText } from '../shared';
import type { BlockDevice, AutoconfigProfile, AutoconfigProfilesChangedDetail, FlashMethod } from '../../types';
import { isEdlMethod } from '../../types';
import { getBlockDevices, getQdlDevices } from '../../hooks/useTauri';
import { getAutoconfigProfiles, getAllowSystemDevices } from '../../hooks/useSettings';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useSkeletonLoading } from '../../hooks/useSkeletonLoading';
import { POLLING, UI, EVENTS, qdlInstructionsKey } from '../../config';
import { getDeviceColors } from '../../config/deviceColors';
import { getDeviceType, devicesChanged, sortDevices, qdlToBlockDevice } from '../../utils/deviceUtils';

/** Window event carrying the opt-in autoconfig profile id (or null) picked at flash time. */
export const AUTOCONFIG_PROFILE_SELECTED_EVENT = 'armbian-autoconfig-profile-selected';

interface DevicePanelProps {
  /** Pick a device, revealing the confirm summary (does not flash yet). */
  onSelect: (device: BlockDevice) => void;
  /** Confirm the summary and start flashing. */
  onConfirm: () => void;
  /** Cancel the summary, back to the device list. */
  onCancel: () => void;
  /** The picked device; when set, the panel shows the confirm summary. */
  selectedDevice: BlockDevice | null;
  /** Flash method for the selected image ("block", "qdl", or "qdl-ufs"). */
  flashMethod?: FlashMethod;
  /** EDL-entry hint ("button"/"jumper") for the selected board, drives the QDL instructions. */
  edlEntry?: string | null;
  /** Upstream selections (manufacturer/board/OS) shown in the confirm summary. */
  summary?: { label: string; value: string }[];
  /** Cached board photo shown at the top of the confirm summary. */
  boardImage?: string | null;
  /** Whether autoconfig profiles apply (Armbian images only; hidden for generic custom images). */
  supportsAutoconfig?: boolean;
}

/** Inline storage browser: device list while picking, confirm summary once `selectedDevice` is set. */
export function DevicePanel({
  onSelect,
  onConfirm,
  onCancel,
  selectedDevice,
  flashMethod,
  edlEntry,
  summary = [],
  boardImage,
  supportsAutoconfig = true,
}: DevicePanelProps) {
  const { t } = useTranslation();
  const [showSystemDevices, setShowSystemDevices] = useState(false);
  // Persistent opt-in to show and flash internal/system disks at the user's own risk.
  const [allowSystemDevices, setAllowSystemDevices] = useState(false);

  useEffect(() => {
    const load = () => {
      getAllowSystemDevices().then(setAllowSystemDevices).catch(() => {});
    };
    load();
    window.addEventListener(EVENTS.SETTINGS_CHANGED, load);
    return () => window.removeEventListener(EVENTS.SETTINGS_CHANGED, load);
  }, []);

  const prevDevicesRef = useRef<BlockDevice[] | null>(null);
  const [devices, setDevices] = useState<BlockDevice[]>([]);

  // Both QDL (TAR) and UFS images flash over EDL, so they share device detection (getQdlDevices).
  const isQdlMode = isEdlMethod(flashMethod);

  // Autoconfig profiles apply to Armbian images (both sd/.img.xz and QDL); hidden for generic custom images.
  const showAutoconfig = supportsAutoconfig;

  // Opt-in autoconfig profile picker (flash-time only, Armbian images).
  const [showProfilePicker, setShowProfilePicker] = useState(false);
  const [selectedProfileId, setSelectedProfileId] = useState('');

  const { data: profilesData, reload: reloadProfiles } = useAsyncData<AutoconfigProfile[]>(
    () => (showAutoconfig ? getAutoconfigProfiles() : Promise.resolve([])),
    [showAutoconfig]
  );
  const profiles = profilesData ?? [];

  // Notify App of the opt-in selection; default "" means no profile (unchanged behaviour).
  const handleProfileChange = useCallback((id: string) => {
    setSelectedProfileId(id);
    // Collapse the picker once a choice is made — the row already shows the result.
    setShowProfilePicker(false);
    window.dispatchEvent(
      new CustomEvent(AUTOCONFIG_PROFILE_SELECTED_EVENT, { detail: { id: id || null } })
    );
  }, []);

  // Keep the picker in sync with profile create/edit/delete done in Settings, without re-mounting.
  useEffect(() => {
    if (!showAutoconfig) return;
    const onProfilesChanged = (e: Event) => {
      const detail = (e as CustomEvent<AutoconfigProfilesChangedDetail>).detail;
      reloadProfiles();
      if (detail?.action === 'deleted' && detail.id === selectedProfileId) {
        handleProfileChange('');
      }
    };
    window.addEventListener(EVENTS.PROFILES_CHANGED, onProfilesChanged);
    return () => window.removeEventListener(EVENTS.PROFILES_CHANGED, onProfilesChanged);
  }, [showAutoconfig, selectedProfileId, reloadProfiles, handleProfileChange]);

  // Auto-select only a profile created from this panel's "Create new" shortcut.
  useEffect(() => {
    const onCreated = (e: Event) => {
      const id = (e as CustomEvent<{ id: string }>).detail?.id;
      if (id) handleProfileChange(id);
    };
    window.addEventListener(EVENTS.AUTOCONFIG_PROFILE_CREATED, onCreated);
    return () => window.removeEventListener(EVENTS.AUTOCONFIG_PROFILE_CREATED, onCreated);
  }, [handleProfileChange]);

  // Open Settings on the profiles tab with the new-profile editor already open.
  const openProfileCreator = useCallback(() => {
    setShowProfilePicker(false);
    window.dispatchEvent(
      new CustomEvent(EVENTS.OPEN_SETTINGS, { detail: { view: 'profiles', createProfile: true } })
    );
  }, []);

  const { data: rawDevices, loading, error, reload } = useAsyncData<BlockDevice[]>(
    async () => {
      if (isQdlMode) {
        const qdlDevices = await getQdlDevices();
        return qdlDevices.map(qdlToBlockDevice);
      }
      return getBlockDevices();
    },
    [isQdlMode]
  );

  // Also "ready" when loading finished with no devices found.
  const devicesReady = useMemo(() => {
    return (devices && devices.length > 0) || !loading;
  }, [devices, loading]);

  // System disks are shown when unlocked via settings, or peeked via the local toggle.
  const showSystem = allowSystemDevices || showSystemDevices;
  const filteredDevices = useMemo(() => {
    const filtered = showSystem ? devices : devices.filter((d) => !d.is_system);
    return sortDevices(filtered);
  }, [devices, showSystem]);

  const { showSkeleton } = useSkeletonLoading(loading, devicesReady);

  // Sync external device data into local state only when it actually changes.
  useEffect(() => {
    if (!rawDevices) return;
    if (devicesChanged(prevDevicesRef.current, rawDevices)) {
      prevDevicesRef.current = rawDevices;
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Sync external device data for change detection
      setDevices(sortDevices(rawDevices));
    }
  }, [rawDevices]);

  // Poll for device changes (detect new USB/SD insertions) while browsing.
  const pollDevices = useCallback(async () => {
    try {
      let newDevices: BlockDevice[];
      if (isQdlMode) {
        const qdlDevices = await getQdlDevices();
        newDevices = qdlDevices.map(qdlToBlockDevice);
      } else {
        newDevices = await getBlockDevices();
      }
      if (devicesChanged(prevDevicesRef.current, newDevices)) {
        prevDevicesRef.current = newDevices;
        setDevices(sortDevices(newDevices));
      }
    } catch {
      // Ignore transient polling errors
    }
  }, [isQdlMode]);

  useEffect(() => {
    if (selectedDevice) return;
    const interval = setInterval(pollDevices, POLLING.DEVICE_CHECK);
    return () => clearInterval(interval);
  }, [selectedDevice, pollDevices]);

  function handleDeviceClick(device: BlockDevice) {
    if (device.is_read_only || (device.is_system && !allowSystemDevices)) return;
    onSelect(device);
  }

  const refreshButton = (
    <button className="device-refresh-btn" onClick={reload} disabled={loading}>
      <RefreshCw size={15} className={loading ? 'spin' : ''} />
      {t('device.refresh')}
    </button>
  );

  // Confirm view: build + target summary, erase warning, and a flash action.
  if (selectedDevice) {
    return (
      <div className="mfr-panel device-panel">
        <div className="device-panel__body device-confirm">
          <div className="device-confirm__inner">
            {boardImage && (
              <div className="device-confirm__board">
                {/* BoardImage owns the logo watermark fallback when no photo exists. */}
                <BoardImage src={boardImage} alt="" />
              </div>
            )}

            <div className="device-confirm__main">
              <h2 className="device-confirm__title">{t('flash.confirmTitle')}</h2>
              <p className="device-confirm__subtitle">{t('flash.confirmText')}</p>

              <ul className="device-summary">
                {summary.map((row) => (
                  <li key={row.label} className="device-summary__row">
                    <span className="device-summary__label">{row.label}</span>
                    {/* Long values (e.g. custom image filenames) auto-scroll instead of truncating. */}
                    <MarqueeText text={row.value} className="device-summary__value" maxWidth={340} />
                  </li>
                ))}
                {/* Target device as a label/value row, consistent with the rows above. */}
                <li className="device-summary__target">
                  <span className="device-summary__label">{t(isQdlMode ? 'home.device' : 'home.storage')}</span>
                  <span className="device-summary__targetinfo">
                    <MarqueeText
                      text={selectedDevice.model || selectedDevice.name}
                      className="device-summary__targetname"
                      maxWidth={300}
                    />
                    <span className="device-summary__targetsub">
                      {selectedDevice.name}
                      {selectedDevice.size_formatted ? ` • ${selectedDevice.size_formatted}` : ''}
                    </span>
                  </span>
                </li>
                {/* System/internal target: contextual danger row tied to the storage above. */}
                {selectedDevice.is_system && (
                  <li className="device-summary__syswarn">
                    <TriangleAlert size={15} />
                    <span>{t('flash.systemDeviceWarning')}</span>
                  </li>
                )}
                {/* Opt-in autoconfig profile, integrated as the final summary row (Armbian images only). */}
                {showAutoconfig && (
                  <li className="device-summary__profile">
                    <button
                      type="button"
                      className={`device-summary__profilerow ${showProfilePicker ? 'is-open' : ''}`}
                      onClick={() => setShowProfilePicker((v) => !v)}
                      aria-expanded={showProfilePicker}
                    >
                      <span className="device-summary__label">{t('flash.profile.rowLabel')}</span>
                      <span className="device-summary__profileval">
                        {selectedProfileId ? (
                          <span className="device-summary__profilename">
                            {profiles.find((p) => p.id === selectedProfileId)?.name ?? ''}
                          </span>
                        ) : (
                          <span className="device-summary__profilenone">{t('flash.profile.none')}</span>
                        )}
                        <ChevronDown size={15} className="device-summary__profilechevron" />
                      </span>
                    </button>

                    {showProfilePicker && (
                      <div className="device-summary__profilebody">
                        {profiles.length > 0 && (
                          <div className="device-profile__select-wrap">
                            <select
                              className="device-profile__select"
                              value={selectedProfileId}
                              onChange={(e) => handleProfileChange(e.target.value)}
                            >
                              {/* First option is the opt-out default (no profile applied). */}
                              <option value="">{t('flash.profile.none')}</option>
                              {profiles.map((p) => (
                                <option key={p.id} value={p.id}>
                                  {p.name}
                                </option>
                              ))}
                            </select>
                            <ChevronDown size={16} className="device-profile__select-chevron" />
                          </div>
                        )}
                        <button type="button" className="device-profile__create" onClick={openProfileCreator}>
                          <Plus size={14} strokeWidth={2.25} />
                          {t('flash.profile.createNew')}
                        </button>
                      </div>
                    )}
                  </li>
                )}
              </ul>

              <div className="device-confirm__warn">
                <TriangleAlert size={15} />
                {t('flash.confirmWarning')}
              </div>

              <div className="device-confirm__actions">
                <button type="button" className="device-confirm__cancel" onClick={onCancel}>
                  {t('common.cancel')}
                </button>
                <button type="button" className="device-confirm__go" onClick={onConfirm}>
                  {t('flash.eraseAndFlash')}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="mfr-panel device-panel">
      <div className="device-panel__head">
        <div className="device-alert">
          <span className="device-alert__icon">
            <TriangleAlert size={16} />
          </span>
          <span className="device-alert__text">{t('flash.dataWarning')}</span>

          {!isQdlMode && !allowSystemDevices && (
            <button
              type="button"
              onClick={() => setShowSystemDevices(!showSystemDevices)}
              className={`device-toggle ${showSystemDevices ? 'is-active' : ''}`}
            >
              <Shield size={14} />
              <span>{showSystemDevices ? t('device.hideSystemDevices') : t('device.showSystemDevices')}</span>
            </button>
          )}
        </div>
      </div>

      {error ? (
        <div className="device-panel__body">
          <ErrorDisplay error={error} onRetry={reload} compact />
        </div>
      ) : (
        <div className="device-panel__body">
          {showSkeleton && (
            <div className="device-grid">
              {Array.from({ length: UI.SKELETON.DEVICE_MODAL }).map((_, i) => (
                <div key={i} className="device-card is-skeleton">
                  <span className="sk-shim dev-sk-icon" />
                  <span className="device-card__info">
                    <span className="sk-shim dev-sk-name" />
                    <span className="sk-shim dev-sk-sub" />
                  </span>
                </div>
              ))}
            </div>
          )}

          {filteredDevices.length === 0 && !showSkeleton && (
            <div className="device-empty">
              <span className="device-empty__icon">{isQdlMode ? <Cpu size={30} /> : <Usb size={30} />}</span>
              <p className="device-empty__title">{isQdlMode ? t('device.qdlNotFound') : t('modal.noDevices')}</p>
              <p className="device-empty__hint">
                {isQdlMode ? t(qdlInstructionsKey(edlEntry)) : t('modal.insertDevice')}
              </p>
              {refreshButton}
            </div>
          )}

          {!showSkeleton && filteredDevices.length > 0 && (
            <>
              <div className="device-grid">
                {filteredDevices.map((device) => {
                  const deviceType = getDeviceType(device);
                  const badge = getDeviceBadge(deviceType, t);
                  const colors = getDeviceColors(deviceType);
                  const isDisabled = device.is_read_only || (device.is_system && !allowSystemDevices);
                  // Unlocked system disk: show its real drive glyph, not the lock (it is selectable).
                  const iconType =
                    device.is_system && allowSystemDevices
                      ? getDeviceType({ ...device, is_system: false })
                      : deviceType;

                  return (
                    <button
                      key={device.path}
                      type="button"
                      className={`device-card ${isDisabled ? 'is-disabled' : ''}`}
                      onClick={() => handleDeviceClick(device)}
                      disabled={isDisabled}
                    >
                      <span
                        className="device-card__icon"
                        style={{ backgroundColor: colors.background, color: colors.text }}
                      >
                        <DeviceIcon type={iconType} size={22} />
                      </span>
                      <span className="device-card__info">
                        <span className="device-card__name">
                          {device.model || device.name}
                          {badge && <span className={`${deviceType}-badge`}>{badge}</span>}
                        </span>
                        <span className="device-card__sub">
                          {device.name}
                          {device.size_formatted ? ` • ${device.size_formatted}` : ''}
                        </span>
                      </span>
                      {device.is_read_only ? (
                        <span className="device-card__lock">
                          <Lock size={11} />
                          {t('device.locked')}
                        </span>
                      ) : (
                        !isDisabled && <ArrowRight className="device-card__arrow" size={18} />
                      )}
                    </button>
                  );
                })}
              </div>
              <div className="device-refresh">{refreshButton}</div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
