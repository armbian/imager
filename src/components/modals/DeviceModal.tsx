import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { HardDrive, RefreshCw, AlertTriangle, Shield, MemoryStick, Usb } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay, ConfirmationDialog, ListItemSkeleton } from '../shared';
import type { BlockDevice } from '../../types';
import { getBlockDevices } from '../../hooks/useTauri';
import { useAsyncDataWhen } from '../../hooks/useAsyncData';
import { POLLING, type DeviceType } from '../../config';
import { getDeviceColors } from '../../config/deviceColors';
import { getDeviceType } from '../../utils/deviceUtils';

/** Get icon component for device type */
function DeviceIcon({ type, size = 24 }: { type: DeviceType; size?: number }) {
  switch (type) {
    case 'system':
      return <Shield size={size} />;
    case 'sd':
      return <MemoryStick size={size} />;
    case 'usb':
      return <Usb size={size} />;
    case 'sata':
    case 'sas':
    case 'nvme':
    default:
      return <HardDrive size={size} />;
  }
}

/** Get badge text for device type */
function getDeviceBadge(type: DeviceType, t: (key: string) => string): string | null {
  switch (type) {
    case 'system':
      return t('device.system');
    case 'sd':
      return t('device.sdCard');
    case 'usb':
      return t('device.usb');
    case 'sata':
      return t('device.sata');
    case 'sas':
      return t('device.sas');
    case 'nvme':
      return t('device.nvme');
    default:
      return null;
  }
}

/** Check if device lists are different (by comparing paths) */
function devicesChanged(prev: BlockDevice[] | null, next: BlockDevice[]): boolean {
  if (!prev) return true;
  if (prev.length !== next.length) return true;
  const prevPaths = new Set(prev.map(d => d.path));
  return next.some(d => !prevPaths.has(d.path));
}

/** Sort devices: removable first, then by size */
function sortDevices(devices: BlockDevice[]): BlockDevice[] {
  return [...devices].sort((a, b) => {
    if (a.is_system !== b.is_system) return a.is_system ? 1 : -1;
    if (a.is_removable !== b.is_removable) return a.is_removable ? -1 : 1;
    return b.size - a.size;
  });
}

interface DeviceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (device: BlockDevice) => void;
}

export function DeviceModal({ isOpen, onClose, onSelect }: DeviceModalProps) {
  const { t } = useTranslation();
  const [selectedDevice, setSelectedDevice] = useState<BlockDevice | null>(null);
  const [showConfirm, setShowConfirm] = useState(false);
  const [showSkeleton, setShowSkeleton] = useState(false);
  const [showSystemDevices, setShowSystemDevices] = useState(false);

  // Track previous devices for change detection
  const prevDevicesRef = useRef<BlockDevice[] | null>(null);
  const [devices, setDevices] = useState<BlockDevice[]>([]);

  // Initial load when modal opens
  const { data: rawDevices, loading, error, reload } = useAsyncDataWhen<BlockDevice[]>(
    isOpen,
    () => getBlockDevices(),
    [isOpen]
  );

  // Derive devices ready state
  const devicesReady = useMemo(() => {
    return devices && devices.length > 0;
  }, [devices]);

  // Filter devices based on showSystemDevices toggle
  const filteredDevices = useMemo(() => {
    if (showSystemDevices) {
      return devices;
    }
    return devices.filter(d => !d.is_system);
  }, [devices, showSystemDevices]);

  // Show skeleton with minimum delay
  useEffect(() => {
    let skeletonTimeout: NodeJS.Timeout;

    if (loading) {
      setShowSkeleton(true);
    } else if (devicesReady || (!loading && devices.length === 0)) {
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
    // eslint-disable-next-line react-hooks/exhaustive-deps -- devicesReady already tracks devices changes; adding devices.length causes re-renders during polling
  }, [loading, devicesReady]);

  // Update devices only when they actually change
  useEffect(() => {
    if (!rawDevices) return;
    if (devicesChanged(prevDevicesRef.current, rawDevices)) {
      prevDevicesRef.current = rawDevices;
      setDevices(sortDevices(rawDevices));
    }
  }, [rawDevices]);

  // Poll for device changes while modal is open
  const pollDevices = useCallback(async () => {
    try {
      const newDevices = await getBlockDevices();
      if (devicesChanged(prevDevicesRef.current, newDevices)) {
        prevDevicesRef.current = newDevices;
        setDevices(sortDevices(newDevices));
      }
    } catch {
      // Silently ignore polling errors
    }
  }, []);

  // Auto-refresh devices while modal is open (detect new USB/SD insertions)
  useEffect(() => {
    if (!isOpen || showConfirm) return;

    const interval = setInterval(pollDevices, POLLING.DEVICE_CHECK);
    return () => clearInterval(interval);
  }, [isOpen, showConfirm, pollDevices]);

  function handleDeviceClick(device: BlockDevice) {
    if (device.is_system) return;
    setSelectedDevice(device);
    setShowConfirm(true);
  }

  function handleConfirm() {
    if (selectedDevice && !selectedDevice.is_system) {
      onSelect(selectedDevice);
      setShowConfirm(false);
    }
  }

  return (
    <>
      <Modal
        isOpen={isOpen && !showConfirm}
        onClose={onClose}
        title={t('modal.selectDevice')}
      >
        <div className="device-warning-banner">
          <div className="device-warning-banner-content">
            <div className="device-warning-banner-icon">
              <AlertTriangle size={16} />
            </div>
            <div className="device-warning-banner-title">
              {t('flash.dataWarning')}
            </div>
          </div>

          <button
            onClick={() => setShowSystemDevices(!showSystemDevices)}
            className={`system-devices-badge ${showSystemDevices ? 'active' : ''}`}
          >
            <Shield size={13} />
            <span>{showSystemDevices ? t('device.hideSystemDevices') : t('device.showSystemDevices')}</span>
          </button>
        </div>

        {error ? (
          <ErrorDisplay error={error} onRetry={reload} compact />
        ) : (
          <>
            {showSkeleton && <ListItemSkeleton count={4} />}
            {filteredDevices.length === 0 && !showSkeleton && (
              <div className="no-results">
                <Usb size={40} />
                <p>{t('modal.noDevices')}</p>
                <p style={{ fontSize: 12, color: 'var(--text-muted)' }}>
                  {t('modal.insertDevice')}
                </p>
                <button className="btn btn-secondary" onClick={reload} disabled={loading} style={{ marginTop: 16 }}>
                  <RefreshCw size={14} className={loading ? 'spin' : ''} />
                  {t('device.refresh')}
                </button>
              </div>
            )}
            <div className="modal-list no-animations">
              {!showSkeleton && filteredDevices.map((device) => {
                const deviceType = getDeviceType(device);
                const badge = getDeviceBadge(deviceType, t);
                return (
                  <button
                    key={device.path}
                    className={`list-item ${device.is_removable ? 'removable' : ''} ${device.is_system ? 'system' : ''}`}
                    onClick={() => handleDeviceClick(device)}
                    disabled={device.is_system}
                    style={{ opacity: device.is_system ? 0.5 : 1 }}
                  >
                    <div className="list-item-icon" style={{
                      backgroundColor: getDeviceColors(deviceType).background,
                      color: getDeviceColors(deviceType).text,
                    }}>
                      <DeviceIcon type={deviceType} size={24} />
                    </div>
                    <div className="list-item-content">
                      <div className="list-item-title">
                        {device.model || device.name}
                        {badge && (
                          <span className={`${deviceType}-badge`} style={{ marginLeft: 8 }}>
                            {badge}
                          </span>
                        )}
                      </div>
                      <div className="list-item-subtitle">
                        {device.name} • {device.size_formatted}
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
            {!showSkeleton && filteredDevices.length > 0 && (
              <div className="modal-refresh-bottom">
                <button className="btn btn-secondary" onClick={reload} disabled={loading}>
                  <RefreshCw size={14} className={loading ? 'spin' : ''} />
                  {t('modal.refreshDevices')}
                </button>
              </div>
            )}
          </>
        )}
      </Modal>

      {/* Confirmation Dialog */}
      <ConfirmationDialog
        isOpen={showConfirm && !!selectedDevice && !selectedDevice.is_system}
        title={t('flash.confirmTitle')}
        message={t('flash.confirmText')}
        warning={t('flash.confirmWarning')}
        confirmText={t('flash.eraseAndFlash')}
        onCancel={() => setShowConfirm(false)}
        onConfirm={handleConfirm}
      >
        {selectedDevice && (
          <div className="confirm-device">
            <strong>{selectedDevice.model || selectedDevice.name}</strong>
            <span>{selectedDevice.name} ({selectedDevice.size_formatted})</span>
          </div>
        )}
      </ConfirmationDialog>
    </>
  );
}
