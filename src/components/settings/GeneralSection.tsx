// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useId, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Gauge, Monitor, Moon, Sparkles, Sun } from 'lucide-react';
import { useTheme, type Theme } from '../../contexts/ThemeContext';
import { useMotion, type MotionMode } from '../../contexts/MotionContext';
import { changeLanguage, getSystemLanguage } from '../../i18n';
import { SUPPORTED_LANGUAGES, AUTO_LANGUAGE_CODE, flagUrl } from '../../config/i18n';
import { EVENTS, SETTINGS } from '../../config';
import {
  getLanguage,
  getShowMotd,
  setShowMotd,
  getShowUpdaterModal,
  setShowUpdaterModal,
  getShowWelcome,
  setShowWelcome,
} from '../../hooks/useSettings';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useSettingToggle } from '../../hooks/useSettingToggle';
import { useToasts } from '../../hooks/useToasts';
import { logWarn } from '../../hooks/useTauri';
import { SettingsHero } from './SettingsHero';
import { SegmentedControl, SelectMenu, SettingsGroup, SettingsRow, ToggleRow, type SelectOption } from './controls';

const THEME_CARDS: { value: Theme; Icon: typeof Sun; labelKey: string }[] = [
  { value: SETTINGS.THEME_MODES.LIGHT, Icon: Sun, labelKey: 'settings.themeLight' },
  { value: SETTINGS.THEME_MODES.DARK, Icon: Moon, labelKey: 'settings.themeDark' },
  { value: SETTINGS.THEME_MODES.AUTO, Icon: Monitor, labelKey: 'settings.themeAuto' },
];

const MOTION_OPTIONS: { value: MotionMode; Icon: typeof Sun; labelKey: string }[] = [
  { value: SETTINGS.MOTION_MODES.FULL, Icon: Sparkles, labelKey: 'settings.motionFull' },
  { value: SETTINGS.MOTION_MODES.REDUCE, Icon: Gauge, labelKey: 'settings.motionReduce' },
  { value: SETTINGS.MOTION_MODES.AUTO, Icon: Monitor, labelKey: 'settings.motionAuto' },
];

function WindowPreview({ mode }: { mode: Theme }) {
  const pane = (tone: 'light' | 'dark') => (
    <span className={`theme-card__pane theme-card__pane--${tone}`}>
      <i className="theme-card__bar">
        <b />
        <b />
        <b />
      </i>
      <i className="theme-card__side">
        <b className="is-on" />
        <b />
        <b />
      </i>
      <i className="theme-card__tiles">
        <b />
        <b className="is-lift" />
        <b />
      </i>
    </span>
  );
  return (
    <span className={`theme-card__window theme-card__window--${mode}`} aria-hidden="true">
      {mode !== SETTINGS.THEME_MODES.DARK && pane('light')}
      {mode !== SETTINGS.THEME_MODES.LIGHT && pane('dark')}
    </span>
  );
}

const readSavedLanguage = () =>
  getLanguage()
    .then((saved) => saved ?? AUTO_LANGUAGE_CODE)
    .catch((error) => {
      logWarn('settings', `Failed to read the saved language: ${error}`);
      return AUTO_LANGUAGE_CODE;
    });

export function GeneralSection() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme, resolvedTheme } = useTheme();
  const { motion, setMotion } = useMotion();
  const { showError } = useToasts();
  const themeHeadingId = useId();
  const { data: savedLanguage } = useAsyncData(readSavedLanguage, []);
  const [pickedLanguage, setPickedLanguage] = useState<string | null>(null);
  const language = pickedLanguage ?? savedLanguage;

  const welcome = useSettingToggle(getShowWelcome, setShowWelcome, {
    fallback: SETTINGS.DEFAULTS.SHOW_WELCOME,
    toastKey: 'settings.toast.welcome',
  });
  const motd = useSettingToggle(getShowMotd, setShowMotd, {
    fallback: SETTINGS.DEFAULTS.SHOW_MOTD,
    toastKey: 'settings.toast.motd',
    event: EVENTS.MOTD_CHANGED,
  });
  const updater = useSettingToggle(getShowUpdaterModal, setShowUpdaterModal, {
    fallback: SETTINGS.DEFAULTS.SHOW_UPDATER_MODAL,
    toastKey: 'settings.toast.updater',
  });

  const { data: systemLanguage } = useAsyncData(getSystemLanguage, []);
  const systemCode = systemLanguage ?? i18n.resolvedLanguage ?? i18n.language;
  const languageOptions = useMemo<SelectOption<string>[]>(
    () =>
      SUPPORTED_LANGUAGES.map((lang) =>
        lang.code === AUTO_LANGUAGE_CODE
          ? {
              value: lang.code,
              label: SUPPORTED_LANGUAGES.find((l) => l.code === systemCode)?.name ?? systemCode,
              hint: t('settings.languageAuto'),
              flagUrl: flagUrl(systemCode),
              badge: t('settings.general.autoBadge'),
            }
          : { value: lang.code, label: lang.name, flagUrl: flagUrl(lang.code) }
      ),
    [systemCode, t]
  );

  const handleLanguageChange = async (code: string) => {
    try {
      await changeLanguage(code);
      setPickedLanguage(code);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
    } catch (error) {
      logWarn('settings', `Failed to change language: ${error}`);
      showError(t('settings.toast.languageError'));
    }
  };

  return (
    <>
      <SettingsHero
        hero="general"
        title={t('settings.nav.general')}
        lead={t('settings.general.lead')}
        heroState={{ theme: resolvedTheme }}
      />

      <section className="set-group" aria-labelledby={themeHeadingId}>
        <h3 id={themeHeadingId} className="set-group__eyebrow">
          {t('settings.theme')}
        </h3>
        <div className="theme-cards" role="group" aria-labelledby={themeHeadingId}>
          {THEME_CARDS.map(({ value, Icon, labelKey }) => (
            <button
              key={value}
              type="button"
              className="theme-card"
              aria-pressed={theme === value}
              onClick={() => setTheme(value)}
            >
              <span className="theme-card__stage">
                <WindowPreview mode={value} />
              </span>
              <span className="theme-card__foot">
                <Icon size={16} className="theme-card__icon" aria-hidden="true" />
                <span className="theme-card__label">{t(labelKey)}</span>
                <span className="theme-card__radio" aria-hidden="true" />
              </span>
            </button>
          ))}
        </div>
      </section>

      <SettingsGroup eyebrow={t('settings.general.animLangEyebrow')}>
        <SettingsRow
          label={t('settings.general.animationLevel')}
          description={t('settings.motionHint')}
          control={
            <SegmentedControl
              value={motion}
              ariaLabel={t('settings.general.animationLevel')}
              onChange={(v) => setMotion(v)}
              options={MOTION_OPTIONS.map(({ value, Icon, labelKey }) => ({
                value,
                label: t(labelKey),
                icon: <Icon aria-hidden="true" />,
              }))}
            />
          }
        />
        <SettingsRow
          label={t('settings.language')}
          description={language === AUTO_LANGUAGE_CODE ? t('settings.general.languageFollowsSystem') : undefined}
          control={
            <SelectMenu
              value={language ?? AUTO_LANGUAGE_CODE}
              disabled={language === null}
              options={languageOptions}
              onChange={handleLanguageChange}
              ariaLabel={t('settings.language')}
              searchable
              searchPlaceholder={t('settings.searchLanguage')}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup eyebrow={t('settings.general.noticesEyebrow')}>
        <ToggleRow
          label={t('settings.notifications.showWelcome')}
          description={t('settings.notifications.showWelcomeDescription')}
          checked={welcome.value ?? false}
          loading={welcome.value === undefined}
          busy={welcome.busy}
          onChange={welcome.set}
        />
        <ToggleRow
          label={t('settings.notifications.showMotd')}
          description={t('settings.notifications.showMotdDescription')}
          checked={motd.value ?? false}
          loading={motd.value === undefined}
          busy={motd.busy}
          onChange={motd.set}
        />
        <ToggleRow
          label={t('settings.notifications.showUpdaterModal')}
          description={t('settings.notifications.showUpdaterModalDescription')}
          checked={updater.value ?? false}
          loading={updater.value === undefined}
          busy={updater.busy}
          onChange={updater.set}
        />
      </SettingsGroup>
    </>
  );
}
