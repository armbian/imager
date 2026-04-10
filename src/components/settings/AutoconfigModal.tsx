import { useState, useEffect } from 'react';
import { X, Wifi, User, Globe, Save, Clock, Lock, MapPin, Network, Shield, Key, Terminal, SquareUser } from 'lucide-react';
import {
  getAutoconfigEnabled, setAutoconfigEnabled,
  getWifiEnabled, setWifiEnabled,
  getWifiSsid, setWifiSsid,
  getWifiKey, setWifiKey,
  getWifiCountryCode, setWifiCountryCode,
  getStaticIpEnabled, setStaticIpEnabled,
  getStaticIp, setStaticIp,
  getStaticMask, setStaticMask,
  getStaticGateway, setStaticGateway,
  getStaticDns, setStaticDns,
  getUserName, setUserName,
  getUserPassword, setUserPassword,
  getUserKey, setUserKey,
  getUserShell, setUserShell,
  getUserRealName, setUserRealName,
  getRootPassword, setRootPassword,
  getRootKey, setRootKey,
  getLocale, setLocale,
  getTimezone, setTimezone,
  generateAutoconfigPayload
} from '../../hooks/useAutoconfig';
import { getSystemInfo } from '../../hooks/useTauri';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { LINUX_LOCALES } from '../../utils/linuxData';
import { COUNTRY_CODES } from '../../utils/countryCodes';

interface AutoconfigModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AutoconfigModal({ isOpen, onClose }: AutoconfigModalProps) {
  // State
  const [enabled, setEnabled] = useState(false);
  const [activeTab, setActiveTab] = useState<'system'|'network'|'users'>('system');
  const [isLinux, setIsLinux] = useState<boolean>(true);
  
  // System
  const [locale, setLocaleState] = useState('');
  const [timezone, setTimezoneState] = useState('');
  
  // Network
  const [wifiEnabled, setWifiEnabledState] = useState(false);
  const [wifiSsid, setWifiSsidState] = useState('');
  const [wifiKey, setWifiKeyState] = useState('');
  const [wifiCc, setWifiCcState] = useState('');
  const [staticIpEnabled, setStaticIpEnabledState] = useState(false);
  const [staticIp, setStaticIpState] = useState('');
  const [staticMask, setStaticMaskState] = useState('');
  const [staticGateway, setStaticGatewayState] = useState('');
  const [staticDns, setStaticDnsState] = useState('');
  
  // Users
  const [userName, setUserNameState] = useState('');
  const [userPass, setUserPassState] = useState('');
  const [userKey, setUserKeyState] = useState('');
  const [userShell, setUserShellState] = useState('bash');
  const [userReal, setUserRealState] = useState('');
  const [rootPass, setRootPassState] = useState('');
  const [rootKey, setRootKeyState] = useState('');

  useEffect(() => {
    if (isOpen) {
      getSystemInfo().then(info => setIsLinux(info.platform === 'linux')).catch(() => setIsLinux(true));
      getAutoconfigEnabled().then(setEnabled);
      getWifiEnabled().then(setWifiEnabledState);
      getWifiSsid().then(setWifiSsidState);
      getWifiKey().then(setWifiKeyState);
      getWifiCountryCode().then(setWifiCcState);
      getStaticIpEnabled().then(setStaticIpEnabledState);
      getStaticIp().then(setStaticIpState);
      getStaticMask().then(setStaticMaskState);
      getStaticGateway().then(setStaticGatewayState);
      getStaticDns().then(setStaticDnsState);
      getUserName().then(setUserNameState);
      getUserPassword().then(setUserPassState);
      getUserKey().then(setUserKeyState);
      getUserShell().then(setUserShellState);
      getUserRealName().then(setUserRealState);
      getRootPassword().then(setRootPassState);
      getRootKey().then(setRootKeyState);
      getLocale().then(setLocaleState);
      getTimezone().then(setTimezoneState);
    }
  }, [isOpen]);

  const handleSave = async () => {
    await setAutoconfigEnabled(enabled);
    await setWifiEnabled(wifiEnabled);
    await setWifiSsid(wifiSsid);
    await setWifiKey(wifiKey);
    await setWifiCountryCode(wifiCc);
    await setStaticIpEnabled(staticIpEnabled);
    await setStaticIp(staticIp);
    await setStaticMask(staticMask);
    await setStaticGateway(staticGateway);
    await setStaticDns(staticDns);
    await setUserName(userName);
    await setUserPassword(userPass);
    await setUserKey(userKey);
    await setUserShell(userShell);
    await setUserRealName(userReal);
    await setRootPassword(rootPass);
    await setRootKey(rootKey);
    await setLocale(locale);
    await setTimezone(timezone);

    if (enabled) {
      const payload = await generateAutoconfigPayload();
      if (payload) {
        try {
          const filePath = await save({
            defaultPath: 'armbian-firstlogin.conf',
            filters: [{
              name: 'Configuration',
              extensions: ['conf']
            }]
          });
          
          if (filePath) {
            await writeTextFile(filePath, payload);
          }
        } catch (e) {
          window.alert("Failed to save via Tauri Dialog: " + String(e));
          console.error("Failed to save configuration via Dialog:", e);
        }
      }
    }
    
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay modal-entering" onClick={onClose}>
      <div className="settings-modal settings-modal-entering" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="settings-modal-header">
          <h2>OS Customization (Autoconfig)</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <div className="settings-modal-body">
          
          {/* Sidebar */}
          <div className="settings-sidebar">
            <button className={`settings-sidebar-item ${activeTab === 'system' ? 'active' : ''}`} onClick={() => setActiveTab('system')}>
              <Globe size={20} /><span>System</span>
            </button>
            <button className={`settings-sidebar-item ${activeTab === 'network' ? 'active' : ''}`} onClick={() => setActiveTab('network')}>
              <Wifi size={20} /><span>Network</span>
            </button>
            <button className={`settings-sidebar-item ${activeTab === 'users' ? 'active' : ''}`} onClick={() => setActiveTab('users')}>
              <User size={20} /><span>Users</span>
            </button>
          </div>

          {/* Content Area */}
          <div className="settings-content" style={{ display: 'flex', flexDirection: 'column' }}>
            
            {/* Global Enable Toggle */}
            <div style={{ paddingBottom: '20px', borderBottom: '1px solid var(--border-color)', marginBottom: '20px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div>
                <span style={{ fontWeight: 'bold', fontSize: '1.1rem' }}>Enable OS Customization</span>
                <p style={{ color: 'var(--text-color-muted)', fontSize: '0.9rem', margin: '4px 0 0' }}>Configure Wi-Fi, Users, and Locales before first boot</p>
              </div>
              <label className="settings-toggle">
                <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
                <span className="settings-toggle-slider" />
              </label>
            </div>

            {/* Tab Views */}
            <div style={{ flex: 1, overflowY: 'auto', opacity: enabled ? 1 : 0.5, pointerEvents: enabled ? 'auto' : 'none', paddingRight: '10px' }}>
              {activeTab === 'system' && (
                <div className="settings-section" style={{ paddingBottom: '20px' }}>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Globe /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">System Locale</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <select className="settings-input" style={{ width: '200px' }} value={locale} onChange={(e) => setLocaleState(e.target.value)}>
                        {LINUX_LOCALES.map((loc) => (
                          <option key={loc} value={loc}>{loc}</option>
                        ))}
                      </select>
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Clock /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Timezone</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <select className="settings-input" style={{ width: '200px' }} value={timezone} onChange={(e) => setTimezoneState(e.target.value)}>
                        {Intl.supportedValuesOf('timeZone').map((tz) => (
                          <option key={tz} value={tz}>{tz}</option>
                        ))}
                      </select>
                    </div>
                  </div>
                </div>
              )}

              {activeTab === 'network' && (
                <div className="settings-section" style={{ paddingBottom: '20px' }}>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Wifi /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Enable Wi-Fi</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <label className="settings-toggle">
                        <input type="checkbox" checked={wifiEnabled} onChange={(e) => setWifiEnabledState(e.target.checked)} />
                        <span className="settings-toggle-slider" />
                      </label>
                    </div>
                  </div>
                  {wifiEnabled && (
                    <>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><Wifi /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">SSID</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <input className="settings-input" style={{ width: '200px' }} value={wifiSsid} onChange={(e) => setWifiSsidState(e.target.value)} />
                        </div>
                      </div>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><Lock /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">Password</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <input className="settings-input" type="password" style={{ width: '200px' }} value={wifiKey} onChange={(e) => setWifiKeyState(e.target.value)} />
                        </div>
                      </div>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><MapPin /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">Country Code</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <select className="settings-input" style={{ width: '200px' }} value={wifiCc} onChange={(e) => setWifiCcState(e.target.value)}>
                            {COUNTRY_CODES.map((code) => (
                              <option key={code} value={code}>{code}</option>
                            ))}
                          </select>
                        </div>
                      </div>
                    </>
                  )}

                  <div className="settings-item" style={{ marginTop: '20px' }}>
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Network /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Enable Static IP</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <label className="settings-toggle">
                        <input type="checkbox" checked={staticIpEnabled} onChange={(e) => setStaticIpEnabledState(e.target.checked)} />
                        <span className="settings-toggle-slider" />
                      </label>
                    </div>
                  </div>
                  {staticIpEnabled && (
                    <>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><Network /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">IP Address</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <input className="settings-input" style={{ width: '200px' }} value={staticIp} onChange={(e) => setStaticIpState(e.target.value)} placeholder="192.168.0.100" />
                        </div>
                      </div>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><Network /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">Subnet Mask</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <input className="settings-input" style={{ width: '200px' }} value={staticMask} onChange={(e) => setStaticMaskState(e.target.value)} placeholder="255.255.255.0" />
                        </div>
                      </div>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><Network /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">Gateway</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <input className="settings-input" style={{ width: '200px' }} value={staticGateway} onChange={(e) => setStaticGatewayState(e.target.value)} placeholder="192.168.0.1" />
                        </div>
                      </div>
                      <div className="settings-item">
                        <div className="settings-item-left">
                          <div className="settings-item-icon"><Network /></div>
                          <div className="settings-item-content">
                            <div className="settings-item-label">DNS Servers</div>
                            <div className="settings-item-description">Space separated</div>
                          </div>
                        </div>
                        <div className="settings-item-control">
                          <input className="settings-input" style={{ width: '200px' }} value={staticDns} onChange={(e) => setStaticDnsState(e.target.value)} placeholder="8.8.8.8 8.8.4.4" />
                        </div>
                      </div>
                    </>
                  )}
                </div>
              )}

              {activeTab === 'users' && (
                <div className="settings-section" style={{ paddingBottom: '20px' }}>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Shield /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Root Password</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <input className="settings-input" type="password" style={{ width: '180px' }} value={rootPass} onChange={(e) => setRootPassState(e.target.value)} />
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Key /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Root SSH Key</div>
                        <div className="settings-item-description">Optional public key</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <input type="text" className="settings-input" style={{ width: '180px' }} value={rootKey} onChange={(e) => setRootKeyState(e.target.value)} placeholder="ssh-rsa AAAAB3..." />
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><User /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Username</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <input className="settings-input" style={{ width: '180px' }} value={userName} onChange={(e) => setUserNameState(e.target.value)} placeholder="armbian" />
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Lock /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">User Password</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <input className="settings-input" type="password" style={{ width: '180px' }} value={userPass} onChange={(e) => setUserPassState(e.target.value)} />
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Key /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">User SSH Key</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <input type="text" className="settings-input" style={{ width: '180px' }} value={userKey} onChange={(e) => setUserKeyState(e.target.value)} placeholder="ssh-rsa AAAAB3..." />
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><SquareUser /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Real Name</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <input className="settings-input" style={{ width: '180px' }} value={userReal} onChange={(e) => setUserRealState(e.target.value)} placeholder="Armbian User" />
                    </div>
                  </div>
                  <div className="settings-item">
                    <div className="settings-item-left">
                      <div className="settings-item-icon"><Terminal /></div>
                      <div className="settings-item-content">
                        <div className="settings-item-label">Default Shell</div>
                      </div>
                    </div>
                    <div className="settings-item-control">
                      <select className="settings-input" style={{ width: '180px' }} value={userShell} onChange={(e) => setUserShellState(e.target.value)}>
                        <option value="bash">bash</option>
                        <option value="zsh">zsh</option>
                      </select>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Bottom Actions */}
            <div style={{ paddingTop: '20px', marginTop: 'auto', borderTop: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '15px' }}>
              {!isLinux && enabled && (
                <div style={{ backgroundColor: 'var(--bg-color-dropdown)', border: '1px solid var(--border-color)', padding: '12px', borderRadius: '6px', fontSize: '0.85rem', color: 'var(--text-color-muted)' }}>
                  <strong style={{ color: 'var(--text-color)' }}>Manual File Placement Required:</strong> Since an ext4 filesystem is required, automatic background configuration is currently only supported natively on Linux. Please place the downloaded <code>armbian-firstlogin.conf</code> file manually on your SD card's partition after flashing!
                </div>
              )}
              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px' }}>
                <button className="btn btn-primary" onClick={handleSave}>
                  <Save size={16} /> Save Settings File
                </button>
              </div>
            </div>
            
          </div>
        </div>
      </div>
    </div>
  );
}
