import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';
import {
  Cpu,
  Monitor,
  Tag,
  Box,
  Github,
  BookOpen,
  AlertCircle,
  MessageSquare,
  type LucideIcon,
} from 'lucide-react';
import { getTauriVersion, getSystemInfo } from '../../hooks/useTauri';
import { LINKS } from '../../config/constants';
import armbianLogo from '../../../src-tauri/icons/icon.png';

/**
 * About section
 *
 * Displays app information including:
 * - Hero with logo and description
 * - Technical details (version, platform, arch, Tauri version)
 * - External links (GitHub, Docs, Issues, Forum)
 */

/**
 * Format platform name for display
 * Maps platform identifiers to user-friendly names
 */
function formatPlatformName(platform: string): string {
  const platformNames: Record<string, string> = {
    macos: 'macOS',
    windows: 'Windows',
    linux: 'Linux'
  };
  return platformNames[platform] || platform;
}

/**
 * Info card component
 *
 * Displays a piece of information with an icon, label, and value.
 * Used in the About section to show app details.
 */
interface InfoCardProps {
  icon: LucideIcon;
  label: string;
  value: string;
}

function InfoCard({ icon: Icon, label, value }: InfoCardProps) {
  return (
    <div className="info-card">
      <Icon size={20} className="info-card-icon" />
      <div className="info-card-content">
        <div className="info-card-label">{label}</div>
        <div className="info-card-value">{value}</div>
      </div>
    </div>
  );
}

/**
 * Link button component
 */
interface LinkButtonProps {
  icon: React.ComponentType<{ className?: string; size?: number }>;
  text: string;
  onClick: () => void;
}

function LinkButton({ icon: Icon, text, onClick }: LinkButtonProps) {
  return (
    <button className="link-button" onClick={onClick}>
      <Icon className="link-button-icon" size={20} />
      <span className="link-button-text">{text}</span>
      <svg className="link-button-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <polyline points="9 18 15 12 9 6"></polyline>
      </svg>
    </button>
  );
}

export function AboutSection() {
  const { t } = useTranslation();
  const [appVersion, setAppVersion] = useState<string>('');
  const [platform, setPlatform] = useState<string>('');
  const [arch, setArch] = useState<string>('');
  const [tauriVersion, setTauriVersion] = useState<string>('');

  useEffect(() => {
    const loadAppInfo = async () => {
      try {
        const [version, tauriVer, systemInfo] = await Promise.all([
          getVersion(),
          getTauriVersion(),
          getSystemInfo()
        ]);
        setAppVersion(version);
        setTauriVersion(tauriVer);

        // Format platform name for display
        setPlatform(formatPlatformName(systemInfo.platform));
        setArch(systemInfo.arch);
      } catch (error) {
        console.error('Failed to load app info:', error);
      }
    };
    loadAppInfo();
  }, []);

  const openLink = (url: string) => {
    open(url);
  };

  return (
    <div className="about-section">
      {/* Hero Section */}
      <div className="about-hero">
        <img src={armbianLogo} alt="Armbian" className="about-logo" />
        <h2 className="about-title">Armbian Imager</h2>
        <p className="about-description">{t('settings.appDescription')}</p>
      </div>

      {/* Technical Info Cards */}
      <div className="about-info-cards">
        <InfoCard icon={Tag} label={t('settings.version')} value={`v${appVersion}`} />
        <InfoCard icon={Monitor} label={t('settings.platform')} value={platform} />
        <InfoCard icon={Cpu} label={t('settings.arch')} value={arch} />
        <InfoCard icon={Box} label={t('settings.tauriVersion')} value={`v${tauriVersion}`} />
      </div>

      {/* Links Section */}
      <div className="about-links">
        <h4>{t('settings.links')}</h4>
        <div className="about-links-grid">
          <LinkButton
            icon={Github}
            text={t('settings.githubRepo')}
            onClick={() => openLink(LINKS.GITHUB_REPO)}
          />
          <LinkButton
            icon={BookOpen}
            text={t('settings.documentation')}
            onClick={() => openLink(LINKS.DOCS)}
          />
          <LinkButton
            icon={AlertCircle}
            text={t('settings.reportIssue')}
            onClick={() => openLink(`${LINKS.GITHUB_REPO}/issues`)}
          />
          <LinkButton
            icon={MessageSquare}
            text={t('settings.community')}
            onClick={() => openLink(LINKS.FORUM)}
          />
        </div>
      </div>
    </div>
  );
}
