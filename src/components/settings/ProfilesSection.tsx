// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { EVENTS, UI } from '../../config';
import { useAsyncData } from '../../hooks/useAsyncData';
import { usePagedGrid, type GridFit } from '../../hooks/usePagedGrid';
import { getAutoconfigProfiles } from '../../hooks/useSettings';
import { useSkeletonLoading } from '../../hooks/useSkeletonLoading';
import type { AutoconfigProfile, LeaveGuard } from '../../types';
import { ProfileWizard } from '../autoconfig';
import { ErrorDisplay, GridPager } from '../shared';
import { NewProfileMenu } from './NewProfileMenu';
import { ProfileCard } from './ProfileCard';
import { ProfileEditor } from './ProfileEditor';
import { SettingsHero } from './SettingsHero';

interface ProfilesSectionProps {
  registerLeaveGuard: (guard: LeaveGuard | null) => void;
  onEditorChange: (open: boolean) => void;
}

const { CARD_H, GAP, MIN_CARD_WIDTH, MIN_ROWS, MAX_PER_PAGE, SPEC_AREAS } = UI.PROFILES;
const LIST_FIT: GridFit = { colMin: MIN_CARD_WIDTH, gap: GAP, minRows: MIN_ROWS };
const LIST_STYLE = { '--profile-col-min': `${MIN_CARD_WIDTH}px` } as CSSProperties;
interface EditorState {
  /** null creates a new profile */
  profile: AutoconfigProfile | null;
}

export function ProfilesSection({ registerLeaveGuard, onEditorChange }: ProfilesSectionProps) {
  const { t } = useTranslation();
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const newButtonRef = useRef<HTMLButtonElement>(null);

  const { data, loading, error, reload } = useAsyncData<AutoconfigProfile[]>(
    async () => [...(await getAutoconfigProfiles())].sort((a, b) => b.updatedAt - a.updatedAt),
    []
  );
  const { showSkeleton } = useSkeletonLoading(loading && data === null, data !== null);
  const profiles = data ?? [];
  const { setPage, pageCount, safePage, pagedItems, pageSize, measureGrid } = usePagedGrid(
    profiles,
    CARD_H + GAP,
    null,
    MAX_PER_PAGE,
    LIST_FIT
  );

  useEffect(() => {
    const onChanged = () => {
      reload();
    };
    window.addEventListener(EVENTS.PROFILES_CHANGED, onChanged);
    return () => window.removeEventListener(EVENTS.PROFILES_CHANGED, onChanged);
  }, [reload]);

  const editorOpen = editor !== null;
  useEffect(() => {
    onEditorChange(editorOpen);
    return () => onEditorChange(false);
  }, [editorOpen, onEditorChange]);

  const closeEditor = useCallback(() => {
    registerLeaveGuard(null);
    setEditor(null);
  }, [registerLeaveGuard]);

  const handleSaved = () => {
    closeEditor();
    setPage(1);
  };

  if (editor) {
    return (
      <ProfileEditor
        profile={editor.profile}
        onSaved={handleSaved}
        onDeleted={closeEditor}
        onCancel={closeEditor}
        registerLeaveGuard={registerLeaveGuard}
      />
    );
  }

  const newButton = (
    <NewProfileMenu triggerRef={newButtonRef} onForm={() => setEditor({ profile: null })} onGuided={() => setWizardOpen(true)} />
  );
  const wizard = wizardOpen && (
    <ProfileWizard
      board={null}
      boardImage={null}
      returnFocusRef={newButtonRef}
      onClose={() => setWizardOpen(false)}
      onCreated={() => setPage(1)}
    />
  );

  const isEmpty = data !== null && !error && profiles.length === 0;
  const pending = data === null && !error;

  if (isEmpty && !showSkeleton) {
    return (
      <div className="profiles-page">
        <SettingsHero
          hero="profilesEmpty"
          title={t('settings.nav.profiles')}
          lead={t('settings.autoconfig.emptyHint')}
          actions={newButton}
        />
        <p className="profiles-hint">{t('settings.profiles.emptyHint')}</p>
        {wizard}
      </div>
    );
  }

  return (
    <div className="profiles-page">
      <SettingsHero
        hero="profiles"
        title={t('settings.nav.profiles')}
        lead={t('settings.profiles.lead')}
        actions={newButton}
        loading={pending || isEmpty}
      />

      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : (
        <div ref={measureGrid} className="profiles-list" style={LIST_STYLE} aria-busy={showSkeleton || pending || undefined}>
          {showSkeleton || pending
            ? Array.from({ length: pageSize }).map((_, i) => (
                <div key={i} className="profile-card is-skeleton">
                  <span className="profile-tile sk-shim" />
                  <span className="profile-card__body">
                    <span className="profile-card__head">
                      <span className="sk-shim profile-sk-name" />
                      <span className="sk-shim profile-sk-scope" />
                    </span>
                    <span className="profile-card__spec">
                      {Array.from({ length: SPEC_AREAS }).map((_, j) => (
                        <span key={j} className="profile-card__fact">
                          <span className="profile-card__glyph sk-shim" />
                          <span className="sk-shim profile-sk-value" />
                        </span>
                      ))}
                    </span>
                  </span>
                </div>
              ))
            : pagedItems.map((profile, index) => (
                <ProfileCard
                  key={profile.id}
                  index={index}
                  profile={profile}
                  onEdit={(p) => setEditor({ profile: p })}
                />
              ))}
        </div>
      )}
      <GridPager compact pageCount={pageCount} page={safePage} onChange={setPage} ariaLabel={t('settings.profiles.pagerLabel')} />

      {wizard}
    </div>
  );
}
