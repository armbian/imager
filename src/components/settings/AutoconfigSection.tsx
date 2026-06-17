import { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Plus, Pencil, Trash2, FileCog, ChevronLeft, ChevronDown,
  Network, Globe, KeyRound, User, SlidersHorizontal,
  Wifi, Lock, Eye, EyeOff, Link2, Clock, MapPin, Router, Server, Cable,
  Terminal, ShieldAlert, UserCircle, Info,
} from 'lucide-react';
import type { AutoconfigConfig, AutoconfigProfile } from '../../types';
import { isHttpUrl } from '../../utils';
import {
  getAutoconfigProfiles,
  upsertAutoconfigProfile,
  deleteAutoconfigProfile,
} from '../../hooks/useSettings';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { useToasts } from '../../hooks/useToasts';
import { EVENTS } from '../../config';
import {
  USER_SHELLS,
  COMMON_LOCALES,
  WIFI_COUNTRY_CODES,
  getTimezones,
  renderPresetPreview,
} from '../../config/autoconfig';

/** Count fields that hold a real value (true booleans or non-empty strings). */
function countSet(values: unknown[]): number {
  return values.filter((v) => v === true || (typeof v === 'string' && v.trim() !== '')).length;
}

type IconType = typeof Network;

/** Labeled control wrapper (label on top, control below). */
function Field({ label, children, error }: { label: string; children: React.ReactNode; error?: string }) {
  return (
    <label className="ac-field">
      <span className="ac-field__label">{label}</span>
      {children}
      {error && <span className="ac-field__error">{error}</span>}
    </label>
  );
}

interface TextInputProps {
  icon?: IconType;
  value: string | undefined;
  onChange: (value: string) => void;
  placeholder?: string;
  mono?: boolean;
  list?: string;
  invalid?: boolean;
  disabled?: boolean;
}

/** Text input with an optional leading icon. */
function TextInput({ icon: Icon, value, onChange, placeholder, mono, list, invalid, disabled }: TextInputProps) {
  return (
    <div className={`ac-input${mono ? ' is-mono' : ''}${invalid ? ' is-invalid' : ''}${disabled ? ' is-disabled' : ''}`}>
      {Icon && <Icon size={15} className="ac-input__icon" />}
      <input
        type="text"
        value={value ?? ''}
        placeholder={placeholder}
        list={list}
        disabled={disabled}
        aria-invalid={invalid || undefined}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}

/** Password input with a reveal toggle. */
function PasswordInput({ value, onChange, placeholder }: { value: string | undefined; onChange: (v: string) => void; placeholder?: string }) {
  const [show, setShow] = useState(false);
  return (
    <div className="ac-input">
      <Lock size={15} className="ac-input__icon" />
      <input
        type={show ? 'text' : 'password'}
        value={value ?? ''}
        placeholder={placeholder}
        autoComplete="new-password"
        onChange={(e) => onChange(e.target.value)}
      />
      <button
        type="button"
        className="ac-input__btn"
        onClick={() => setShow((s) => !s)}
        aria-label={show ? 'Hide' : 'Show'}
      >
        {show ? <EyeOff size={15} /> : <Eye size={15} />}
      </button>
    </div>
  );
}

interface SelectInputProps {
  icon?: IconType;
  value: string;
  onChange: (value: string) => void;
  children: React.ReactNode;
  disabled?: boolean;
}

/** Native select styled to match the text inputs, with a leading icon. */
function SelectInput({ icon: Icon, value, onChange, children, disabled }: SelectInputProps) {
  return (
    <div className={`ac-input is-select${disabled ? ' is-disabled' : ''}`}>
      {Icon && <Icon size={15} className="ac-input__icon" />}
      <select value={value} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
        {children}
      </select>
      <ChevronDown size={15} className="ac-input__chevron" />
    </div>
  );
}

interface ToggleRowProps {
  icon?: IconType;
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** Full-width toggle row with optional icon and description. */
function ToggleRow({ icon: Icon, label, description, checked, onChange, disabled }: ToggleRowProps) {
  return (
    <div className={`ac-toggle${disabled ? ' is-disabled' : ''}`}>
      <div className="ac-toggle__text">
        {Icon && <Icon size={16} className="ac-toggle__icon" />}
        <span>
          <span className="ac-toggle__label">{label}</span>
          {description && <span className="ac-toggle__desc">{description}</span>}
        </span>
      </div>
      <label className="toggle-switch">
        <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} aria-label={label} />
        <span className="toggle-slider"></span>
      </label>
    </div>
  );
}

interface SegmentedProps {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}

/** Pill segmented control used for short enum choices (e.g. login shell). */
function Segmented({ value, options, onChange }: SegmentedProps) {
  return (
    <div className="ac-segmented" role="group">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className={`ac-seg${value === o.value ? ' is-active' : ''}`}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** Section card grouping related fields under an icon + title + count badge. */
function SectionCard({
  icon: Icon, title, count, children,
}: { icon: IconType; title: string; count: number; children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <section className="ac-card">
      <button
        type="button"
        className={`ac-card__head ac-card__toggle${open ? ' is-open' : ''}`}
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <span className="ac-card__chip"><Icon size={16} /></span>
        <h4 className="ac-card__title">{title}</h4>
        {count > 0 && <span className="ac-card__count">{count}</span>}
        <ChevronDown size={16} className="ac-card__chevron" />
      </button>
      {open && <div className="ac-card__body">{children}</div>}
    </section>
  );
}

interface AutoconfigSectionProps {
  autoCreate?: boolean;
  onSaved?: () => void;
}

// Profiles tab: lists saved autoconfig profiles (master) and edits one (detail).
export function AutoconfigSection({ autoCreate = false, onSaved }: AutoconfigSectionProps) {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const [profiles, setProfiles] = useState<AutoconfigProfile[]>([]);
  const [loaded, setLoaded] = useState(false);

  // Editor state: null = list view; otherwise editing this draft.
  const [draft, setDraft] = useState<AutoconfigProfile | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<AutoconfigProfile | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const [revealSecrets, setRevealSecrets] = useState(false);

  const timezones = useMemo(() => getTimezones(), []);

  const loadProfiles = useCallback(async () => {
    try {
      const list = await getAutoconfigProfiles();
      // Most recently edited first.
      setProfiles([...list].sort((a, b) => b.updatedAt - a.updatedAt));
    } catch (error) {
      console.error('Failed to load autoconfig profiles:', error);
    } finally {
      setLoaded(true);
    }
  }, []);

  useEffect(() => {
    loadProfiles();
  }, [loadProfiles]);

  // Opened via the "create new profile" shortcut: jump straight into the editor.
  useEffect(() => {
    if (autoCreate) handleNew();
  }, [autoCreate]);

  /** Opens the editor for a new, empty profile. */
  const handleNew = () => {
    setDraft({ id: crypto.randomUUID(), name: '', updatedAt: Date.now(), config: {} });
    setIsNew(true);
    setShowPreview(false);
    setRevealSecrets(false);
  };

  /** Opens the editor for an existing profile (clone so edits stay local until saved). */
  const handleEdit = (profile: AutoconfigProfile) => {
    setDraft({ ...profile, config: { ...profile.config } });
    setIsNew(false);
    setShowPreview(false);
    setRevealSecrets(false);
  };

  const handleCancel = () => {
    setDraft(null);
    setIsNew(false);
  };

  /** Persists the current draft, refreshing the list and notifying listeners. */
  const handleSave = async () => {
    if (!draft) return;
    const name = draft.name.trim();
    if (!name) {
      showError(t('settings.autoconfig.toastError'));
      return;
    }
    try {
      const wasNew = isNew;
      const toSave: AutoconfigProfile = { ...draft, name, updatedAt: Date.now() };
      await upsertAutoconfigProfile(toSave);
      await loadProfiles();
      showSuccess(wasNew ? t('settings.autoconfig.toastCreated') : t('settings.autoconfig.toastUpdated'));
      setDraft(null);
      setIsNew(false);
      // Only a profile created from the flash flow's shortcut should auto-select in the picker.
      if (autoCreate && wasNew) {
        window.dispatchEvent(new CustomEvent(EVENTS.AUTOCONFIG_PROFILE_CREATED, { detail: { id: toSave.id } }));
      }
      onSaved?.();
    } catch (error) {
      console.error('Failed to save autoconfig profile:', error);
      showError(t('settings.autoconfig.toastError'));
    }
  };

  /** Removes a profile after user confirmation. */
  const handleDeleteConfirm = async () => {
    if (!pendingDelete) return;
    try {
      await deleteAutoconfigProfile(pendingDelete.id);
      await loadProfiles();
      showSuccess(t('settings.autoconfig.toastDeleted'));
    } catch (error) {
      console.error('Failed to delete autoconfig profile:', error);
      showError(t('settings.autoconfig.toastError'));
    } finally {
      setPendingDelete(null);
    }
  };

  /** Updates a single config field on the current draft. */
  const setConfig = useCallback(<K extends keyof AutoconfigConfig>(key: K, value: AutoconfigConfig[K]) => {
    setDraft((prev) => (prev ? { ...prev, config: { ...prev.config, [key]: value } } : prev));
  }, []);

  if (!loaded) return null;

  // Detail view: profile editor.
  if (draft) {
    const c = draft.config;
    const preview = renderPresetPreview(c, revealSecrets);

    // Locale/timezone are applied by Armbian only during first-user creation,
    // so they stay locked until the first user is fully defined (name + password + full name).
    const hasUser = !!(c.userName?.trim() && c.userPassword?.trim() && c.userRealName?.trim());
    // Counts mirror what render actually emits: hidden/locked sub-fields don't count.
    const netCount = c.applyNetwork
      ? countSet([
        c.applyNetwork, c.ethernetEnabled, c.wifiEnabled,
        ...(c.wifiEnabled ? [c.wifiSsid, c.wifiKey, c.wifiCountryCode] : []),
        c.useStaticIp,
        ...(c.useStaticIp ? [c.staticIp, c.staticMask, c.staticGateway, c.staticDns] : []),
      ])
      : 0;
    const localeCount = hasUser ? countSet([c.locale, c.timezone, c.langBasedOnLocation]) : 0;
    const rootCount = countSet([c.rootPassword, c.rootKeyUrl]);
    const userCount = countSet([c.userName, c.userPassword, c.userRealName, c.userKeyUrl, c.userShell]);
    const advCount = countSet([c.remoteConfigUrl]);

    return (
      <div className="settings-section ac-editor">
        <div className="autoconfig-editor-header">
          <button className="btn btn-secondary btn-sm" onClick={handleCancel}>
            <ChevronLeft size={16} />
            {t('settings.autoconfig.cancel')}
          </button>
          <span className="ac-editor__name">{draft.name.trim() || t('settings.autoconfig.newProfile')}</span>
          <button className="btn btn-primary btn-sm" onClick={handleSave}>
            {t('settings.autoconfig.save')}
          </button>
        </div>

        {/* Only the form body scrolls; the header above stays fixed. */}
        <div className="ac-editor__body">
        <div className="ac-warning">
          <ShieldAlert size={16} />
          <span>{t('settings.autoconfig.secretWarning')}</span>
        </div>

        <div className="ac-hero">
          <Field label={t('settings.autoconfig.nameLabel')}>
            <TextInput
              icon={FileCog}
              value={draft.name}
              placeholder={t('settings.autoconfig.namePlaceholder')}
              onChange={(v) => setDraft((prev) => (prev ? { ...prev, name: v } : prev))}
            />
          </Field>
        </div>

        <SectionCard icon={Network} title={t('settings.autoconfig.groupNetwork')} count={netCount}>
          <ToggleRow
            icon={Network}
            label={t('settings.autoconfig.applyNetwork')}
            checked={!!c.applyNetwork}
            onChange={(v) => setConfig('applyNetwork', v)}
          />
          {c.applyNetwork && (
            <div className="ac-reveal">
              <ToggleRow
                icon={Cable}
                label={t('settings.autoconfig.ethernetEnabled')}
                checked={!!c.ethernetEnabled}
                onChange={(v) => setConfig('ethernetEnabled', v)}
              />
              <ToggleRow
                icon={Wifi}
                label={t('settings.autoconfig.wifiEnabled')}
                checked={!!c.wifiEnabled}
                onChange={(v) => setConfig('wifiEnabled', v)}
              />
              {c.wifiEnabled && (
                <div className="ac-grid">
                  <Field label={t('settings.autoconfig.wifiSsid')}>
                    <TextInput icon={Wifi} value={c.wifiSsid} placeholder="MyHomeWiFi" onChange={(v) => setConfig('wifiSsid', v)} />
                  </Field>
                  <Field label={t('settings.autoconfig.wifiKey')}>
                    <PasswordInput value={c.wifiKey} onChange={(v) => setConfig('wifiKey', v)} />
                  </Field>
                  <Field label={t('settings.autoconfig.wifiCountryCode')}>
                    <SelectInput
                      icon={Globe}
                      value={c.wifiCountryCode ?? ''}
                      onChange={(v) => setConfig('wifiCountryCode', v || undefined)}
                    >
                      <option value="">{t('settings.autoconfig.selectPlaceholder')}</option>
                      {WIFI_COUNTRY_CODES.map((country) => (
                        <option key={country.code} value={country.code}>
                          {country.code} - {country.name}
                        </option>
                      ))}
                    </SelectInput>
                  </Field>
                </div>
              )}
              <ToggleRow
                icon={MapPin}
                label={t('settings.autoconfig.useStaticIp')}
                checked={!!c.useStaticIp}
                onChange={(v) => setConfig('useStaticIp', v)}
              />
              {c.useStaticIp && (
                <div className="ac-grid">
                  <Field label={t('settings.autoconfig.staticIp')}>
                    <TextInput icon={MapPin} value={c.staticIp} placeholder="192.168.1.50" onChange={(v) => setConfig('staticIp', v)} />
                  </Field>
                  <Field label={t('settings.autoconfig.staticMask')}>
                    <TextInput icon={MapPin} value={c.staticMask} placeholder="255.255.255.0" onChange={(v) => setConfig('staticMask', v)} />
                  </Field>
                  <Field label={t('settings.autoconfig.staticGateway')}>
                    <TextInput icon={Router} value={c.staticGateway} placeholder="192.168.1.1" onChange={(v) => setConfig('staticGateway', v)} />
                  </Field>
                  <Field label={t('settings.autoconfig.staticDns')}>
                    <TextInput icon={Server} value={c.staticDns} placeholder="8.8.8.8, 1.1.1.1" onChange={(v) => setConfig('staticDns', v)} />
                  </Field>
                </div>
              )}
            </div>
          )}
        </SectionCard>

        <SectionCard icon={Globe} title={t('settings.autoconfig.groupLocalization')} count={localeCount}>
          {/* Armbian's firstlogin applies locale/timezone only inside the first-user
              creation step, so these are locked until a username is set. */}
          {!hasUser && (
            <div className="ac-note">
              <Info size={14} />
              <span>{t('settings.autoconfig.localeRequiresUser')}</span>
            </div>
          )}
          <div className="ac-grid">
            <Field label={t('settings.autoconfig.locale')}>
              <TextInput icon={Globe} value={c.locale} placeholder="en_US.UTF-8" list="ac-locales" disabled={!hasUser} onChange={(v) => setConfig('locale', v || undefined)} />
              <datalist id="ac-locales">
                {COMMON_LOCALES.map((loc) => <option key={loc} value={loc} />)}
              </datalist>
            </Field>
            <Field label={t('settings.autoconfig.timezone')}>
              <SelectInput icon={Clock} value={c.timezone ?? ''} disabled={!hasUser} onChange={(v) => setConfig('timezone', v || undefined)}>
                <option value="">{t('settings.autoconfig.selectPlaceholder')}</option>
                {timezones.map((tz) => <option key={tz} value={tz}>{tz}</option>)}
              </SelectInput>
            </Field>
          </div>
          <ToggleRow
            icon={MapPin}
            label={t('settings.autoconfig.langBasedOnLocation')}
            checked={!!c.langBasedOnLocation}
            disabled={!hasUser}
            onChange={(v) => setConfig('langBasedOnLocation', v)}
          />
        </SectionCard>

        <SectionCard icon={KeyRound} title={t('settings.autoconfig.groupRoot')} count={rootCount}>
          <div className="ac-grid">
            <Field label={t('settings.autoconfig.rootPassword')}>
              <PasswordInput value={c.rootPassword} placeholder={t('settings.autoconfig.passwordHint')} onChange={(v) => setConfig('rootPassword', v)} />
            </Field>
            <Field
              label={t('settings.autoconfig.rootKeyUrl')}
              error={c.rootKeyUrl && !isHttpUrl(c.rootKeyUrl) ? t('settings.autoconfig.keyUrlInvalid') : undefined}
            >
              <TextInput
                icon={Link2}
                mono
                value={c.rootKeyUrl}
                invalid={!!c.rootKeyUrl && !isHttpUrl(c.rootKeyUrl)}
                placeholder="https://github.com/username.keys"
                onChange={(v) => setConfig('rootKeyUrl', v)}
              />
            </Field>
          </div>
        </SectionCard>

        <SectionCard icon={User} title={t('settings.autoconfig.groupUser')} count={userCount}>
          <div className="ac-grid">
            <Field label={t('settings.autoconfig.userName')}>
              <TextInput icon={User} value={c.userName} placeholder="armbian" onChange={(v) => setConfig('userName', v)} />
            </Field>
            <Field label={t('settings.autoconfig.userPassword')}>
              <PasswordInput value={c.userPassword} placeholder={t('settings.autoconfig.passwordHint')} onChange={(v) => setConfig('userPassword', v)} />
            </Field>
            <Field label={t('settings.autoconfig.userRealName')}>
              <TextInput icon={UserCircle} value={c.userRealName} placeholder="Armbian User" onChange={(v) => setConfig('userRealName', v)} />
            </Field>
            <Field
              label={t('settings.autoconfig.userKeyUrl')}
              error={c.userKeyUrl && !isHttpUrl(c.userKeyUrl) ? t('settings.autoconfig.keyUrlInvalid') : undefined}
            >
              <TextInput
                icon={Link2}
                mono
                value={c.userKeyUrl}
                invalid={!!c.userKeyUrl && !isHttpUrl(c.userKeyUrl)}
                placeholder="https://github.com/username.keys"
                onChange={(v) => setConfig('userKeyUrl', v)}
              />
            </Field>
          </div>
          <Field label={t('settings.autoconfig.userShell')}>
            <Segmented
              value={c.userShell ?? ''}
              options={[
                { value: '', label: '—' },
                ...USER_SHELLS.map((s) => ({ value: s, label: s })),
              ]}
              onChange={(v) => setConfig('userShell', (v || undefined) as AutoconfigConfig['userShell'])}
            />
          </Field>
          {/* Armbian only skips first-boot user creation when a username is set. */}
          <p className="ac-hint">{t('settings.autoconfig.userHint')}</p>
        </SectionCard>

        <SectionCard icon={SlidersHorizontal} title={t('settings.autoconfig.groupAdvanced')} count={advCount}>
          <Field
            label={t('settings.autoconfig.remoteConfigUrl')}
            error={c.remoteConfigUrl && !isHttpUrl(c.remoteConfigUrl) ? t('settings.autoconfig.urlInvalid') : undefined}
          >
            <TextInput
              icon={Link2}
              mono
              value={c.remoteConfigUrl}
              invalid={!!c.remoteConfigUrl && !isHttpUrl(c.remoteConfigUrl)}
              placeholder="https://example.com/config.txt"
              onChange={(v) => setConfig('remoteConfigUrl', v)}
            />
          </Field>
        </SectionCard>

        {/* Live preview of the exact file the backend will write on first boot. */}
        <section className="ac-card ac-preview">
          <button
            type="button"
            className={`ac-card__head ac-preview__toggle${showPreview ? ' is-open' : ''}`}
            onClick={() => setShowPreview((v) => !v)}
            aria-expanded={showPreview}
          >
            <span className="ac-card__chip"><Terminal size={16} /></span>
            <h4 className="ac-card__title">{t('settings.autoconfig.previewTitle')}</h4>
            {preview.count > 0 && <span className="ac-card__count">{preview.count}</span>}
            <ChevronDown size={16} className="ac-preview__chevron" />
          </button>
          {showPreview && (
            <div className="ac-preview__body">
              <div className="ac-preview__bar">
                <span className="ac-preview__path">/root/.not_logged_in_yet</span>
                <button type="button" className="ac-preview__reveal" onClick={() => setRevealSecrets((v) => !v)}>
                  {revealSecrets ? <EyeOff size={13} /> : <Eye size={13} />}
                  {revealSecrets ? t('settings.autoconfig.previewHide') : t('settings.autoconfig.previewReveal')}
                </button>
              </div>
              {preview.count > 0 ? (
                <pre className="ac-preview__code">{preview.content}</pre>
              ) : (
                <p className="ac-preview__empty">{t('settings.autoconfig.previewEmpty')}</p>
              )}
            </div>
          )}
        </section>
        </div>
      </div>
    );
  }

  // Master view: profile list.
  return (
    <div className="settings-section">
      <div className="settings-group">
        <div className="autoconfig-list-header">
          <div className="settings-row__text">
            <div className="settings-group__title">{t('settings.autoconfig.title')}</div>
            <div className="settings-row__desc">{t('settings.autoconfig.description')}</div>
          </div>
          <button className="btn btn-primary btn-sm" onClick={handleNew}>
            <Plus size={16} />
            {t('settings.autoconfig.newProfile')}
          </button>
        </div>

        {profiles.length === 0 ? (
          <div className="autoconfig-empty">
            <FileCog size={32} />
            <div className="autoconfig-empty-title">{t('settings.autoconfig.emptyTitle')}</div>
            <div className="autoconfig-empty-hint">{t('settings.autoconfig.emptyHint')}</div>
          </div>
        ) : (
          <div className="settings-group__card">
            {profiles.map((profile) => (
              <div key={profile.id} className="settings-row">
                <div className="settings-row__main">
                  <div className="settings-row__icon">
                    <FileCog size={18} />
                  </div>
                  <div className="settings-row__text">
                    <div className="settings-row__label">{profile.name}</div>
                    <div className="settings-row__desc">
                      {renderPresetPreview(profile.config).count} {t('settings.autoconfig.settingsCount')}
                    </div>
                  </div>
                </div>
                <div className="autoconfig-item-actions">
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => handleEdit(profile)}
                    aria-label={t('settings.autoconfig.edit')}
                  >
                    <Pencil size={16} />
                    {t('settings.autoconfig.edit')}
                  </button>
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => setPendingDelete(profile)}
                    aria-label={t('settings.autoconfig.delete')}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <ConfirmationDialog
        isOpen={pendingDelete !== null}
        title={t('settings.autoconfig.deleteConfirmTitle')}
        message={t('settings.autoconfig.deleteConfirmBody', { name: pendingDelete?.name ?? '' })}
        confirmText={t('settings.autoconfig.delete')}
        isDanger={true}
        onCancel={() => setPendingDelete(null)}
        onConfirm={handleDeleteConfirm}
      />
    </div>
  );
}
