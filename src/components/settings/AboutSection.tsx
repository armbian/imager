import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';
import {
  Cpu,
  Monitor,
  Tag,
  Box,
  ChevronRight,
  Github,
  BookOpen,
  CircleAlert,
  MessageSquare,
  type LucideIcon,
} from 'lucide-react';
import { getTauriVersion, getSystemInfo } from '../../hooks/useTauri';
import { LINKS } from '../../config/constants';
import armbianLogo from '../../../src-tauri/icons/icon.png';

/** Maps a raw backend platform id (e.g. "macos") to a display name (e.g. "macOS"); returns the original when unknown. */
function formatPlatformName(platform: string): string {
  const platformNames: Record<string, string> = {
    macos: 'macOS',
    windows: 'Windows',
    linux: 'Linux',
  };
  return platformNames[platform] || platform;
}

interface InfoCardProps {
  /** Leading icon rendered inside the accent-tinted chip. */
  icon: LucideIcon;
  /** Muted label describing the value. */
  label: string;
  /** Resolved value to display (already formatted). */
  value: string;
}

/** Glass info card surfacing a single piece of build/environment metadata. */
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

interface LinkButtonProps {
  /** Leading icon for the link row. */
  icon: LucideIcon;
  /** Visible link label. */
  text: string;
  /** Invoked on click; opens the external URL via the shell. */
  onClick: () => void;
}

/** Glass list row that opens an external resource, with a hover-sliding arrow. */
function LinkButton({ icon: Icon, text, onClick }: LinkButtonProps) {
  return (
    <button className="link-button" onClick={onClick}>
      <Icon className="link-button-icon" size={20} />
      <span className="link-button-text">{text}</span>
      <ChevronRight className="link-button-arrow" size={20} />
    </button>
  );
}

/** About tab: branding hero, env info cards (app/Tauri version, platform, arch), and external links. Metadata is
 * fetched in parallel on mount; failures are log-only (cards stay empty). */
export function AboutSection() {
  const { t } = useTranslation();
  const [appVersion, setAppVersion] = useState<string>('');
  const [platform, setPlatform] = useState<string>('');
  const [arch, setArch] = useState<string>('');
  const [tauriVersion, setTauriVersion] = useState<string>('');

  // Load app, Tauri and system metadata in parallel; log-only on failure.
  useEffect(() => {
    const loadAppInfo = async () => {
      try {
        const [version, tauriVer, systemInfo] = await Promise.all([
          getVersion(),
          getTauriVersion(),
          getSystemInfo(),
        ]);
        setAppVersion(version);
        setTauriVersion(tauriVer);
        setPlatform(formatPlatformName(systemInfo.platform));
        setArch(systemInfo.arch);
      } catch (error) {
        console.error('Failed to load app info:', error);
      }
    };
    loadAppInfo();
  }, []);

  /** Opens an external URL in the user's default browser via the shell. */
  const openLink = (url: string) => {
    open(url);
  };

  return (
    <div className="about-section">
      {/* Branding hero: floating logo over an accent-tint bloom. */}
      <div className="about-hero">
        <img src={armbianLogo} alt="Armbian" className="about-logo" />
        <h2 className="about-title">Armbian Imager</h2>
        <p className="about-description">{t('settings.appDescription')}</p>
      </div>

      {/* Environment metadata as a grid of glass info cards. */}
      <div className="about-info-cards">
        <InfoCard icon={Tag} label={t('settings.version')} value={`v${appVersion}`} />
        <InfoCard icon={Monitor} label={t('settings.platform')} value={platform} />
        <InfoCard icon={Cpu} label={t('settings.arch')} value={arch} />
        <InfoCard icon={Box} label={t('settings.tauriVersion')} value={`v${tauriVersion}`} />
      </div>

      {/* External resource links as a grid of glass list rows. */}
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
            icon={CircleAlert}
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
