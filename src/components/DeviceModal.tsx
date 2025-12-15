import { useState, useEffect, useRef, useCallback } from 'react';
import { HardDrive, RefreshCw, AlertTriangle, Shield, MemoryStick, Usb } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Modal } from './Modal';
import { ErrorDisplay } from './shared/ErrorDisplay';
import type { BlockDevice } from '../types';
import { getBlockDevices } from '../hooks/useTauri';
import { useAsyncDataWhen } from '../hooks/useAsyncData';

/** Polling interval for device detection (ms) */
const DEVICE_POLL_INTERVAL = 2000;

/** Device type for icon selection */
type DeviceType = 'sd' | 'usb' | 'sata' | 'sas' | 'nvme' | 'hdd' | 'system';

/** Determine device type based on bus_type, model, and path */
function getDeviceType(device: BlockDevice): DeviceType {
  if (device.is_system) return 'system';

  const busType = (device.bus_type || '').toUpperCase();
  const model = (device.model || '').toLowerCase();
  const path = (device.path || '').toLowerCase();

  // Primary: use bus_type from backend (most reliable)
  if (busType === 'SD' || busType === 'MMC') {
    return 'sd';
  }
  if (busType === 'USB') {
    return 'usb';
  }
  if (busType === 'SATA') {
    return 'sata';
  }
  if (busType === 'SAS') {
    return 'sas';
  }
  if (busType === 'NVME') {
    return 'nvme';
  }

  // Fallback: detect from path (Linux mmcblk, nvme)
  if (path.includes('mmcblk')) {
    return 'sd';
  }
  if (path.includes('nvme')) {
    return 'nvme';
  }

  // Fallback: detect from model name
  // Match: "sdxc", "sdhc", "sd card", "card reader" - but NOT "ssd"
  const isSDCard =
    model.includes('sdxc') ||
    model.includes('sdhc') ||
    model.includes('card reader') ||
    model.includes('sd reader') ||
    model.includes('sd card');

  if (isSDCard) {
    return 'sd';
  }

  // USB detection - removable drives that aren't SD cards
  if (device.is_removable) {
    return 'usb';
  }

  return 'hdd';
}

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

  // Track previous devices for change detection
  const prevDevicesRef = useRef<BlockDevice[] | null>(null);
  const [devices, setDevices] = useState<BlockDevice[]>([]);

  // Initial load when modal opens
  const { data: rawDevices, loading, error, reload } = useAsyncDataWhen<BlockDevice[]>(
    isOpen,
    () => getBlockDevices(),
    [isOpen]
  );

  // Update devices only when they actually change
  useEffect(() => {
    if (!rawDevices) return;
    if (devicesChanged(prevDevicesRef.current, rawDevices)) {
      prevDevicesRef.current = rawDevices;
      // eslint-disable-next-line react-hooks/set-state-in-effect -- Sync external data to state
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

    const interval = setInterval(pollDevices, DEVICE_POLL_INTERVAL);
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
      <Modal isOpen={isOpen && !showConfirm} onClose={onClose} title={t('modal.selectDevice')}>
        <div className="modal-warning-banner">
          <AlertTriangle size={16} />
          <span>{t('flash.dataWarning')}</span>
        </div>

        {loading ? (
          <div className="loading">
            <div className="spinner" />
            <p>{t('modal.scanningDevices')}</p>
          </div>
        ) : error ? (
          <ErrorDisplay error={error} onRetry={reload} compact />
        ) : devices.length === 0 ? (
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
        ) : (
          <>
            <div className="modal-list">
              {devices.map((device) => {
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
                      backgroundColor: deviceType === 'system' ? 'rgba(239, 68, 68, 0.1)' :
                        deviceType === 'sd' ? 'rgba(59, 130, 246, 0.1)' :
                        deviceType === 'usb' ? 'rgba(16, 185, 129, 0.1)' :
                        deviceType === 'sata' ? 'rgba(249, 115, 22, 0.1)' :
                        deviceType === 'sas' ? 'rgba(168, 85, 247, 0.1)' :
                        deviceType === 'nvme' ? 'rgba(236, 72, 153, 0.1)' : 'var(--bg-secondary)',
                      color: deviceType === 'system' ? '#ef4444' :
                        deviceType === 'sd' ? '#3b82f6' :
                        deviceType === 'usb' ? '#10b981' :
                        deviceType === 'sata' ? '#f97316' :
                        deviceType === 'sas' ? '#a855f7' :
                        deviceType === 'nvme' ? '#ec4899' : 'var(--text-secondary)'
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
            <div className="modal-refresh-bottom">
              <button className="btn btn-secondary" onClick={reload} disabled={loading}>
                <RefreshCw size={14} className={loading ? 'spin' : ''} />
                {t('modal.refreshDevices')}
              </button>
            </div>
          </>
        )}
      </Modal>

      {/* Confirmation Dialog */}
      {showConfirm && selectedDevice && !selectedDevice.is_system && (
        <div className="modal-overlay" onClick={() => setShowConfirm(false)}>
          <div className="modal confirm-modal" onClick={(e) => e.stopPropagation()}>
            <div className="confirm-icon">
              <AlertTriangle size={32} color="#f59e0b" />
            </div>
            <h3 className="confirm-title">{t('flash.confirmTitle')}</h3>
            <p className="confirm-text">{t('flash.confirmText')}</p>
            <div className="confirm-device">
              <strong>{selectedDevice.model || selectedDevice.name}</strong>
              <span>{selectedDevice.name} ({selectedDevice.size_formatted})</span>
            </div>
            <p className="confirm-warning">{t('flash.confirmWarning')}</p>
            <div className="confirm-actions">
              <button className="btn btn-secondary" onClick={() => setShowConfirm(false)}>
                {t('flash.cancel')}
              </button>
              <button className="btn btn-danger" onClick={handleConfirm}>
                {t('flash.eraseAndFlash')}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
