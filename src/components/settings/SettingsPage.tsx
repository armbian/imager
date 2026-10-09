// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useRef, useState } from 'react';
import { UI } from '../../config';
import { useMotion } from '../../contexts/MotionContext';
import type { LeaveGuard, SettingsView } from '../../types';
import { SettingsNav } from './SettingsNav';
import { GeneralSection } from './GeneralSection';
import { WritingSection } from './WritingSection';
import { ProfilesSection } from './ProfilesSection';
import { DownloadsSection } from './DownloadsSection';
import { AboutSection } from './AboutSection';

interface SettingsPageProps {
  view: SettingsView;
  exiting: boolean;
  /** Guarded by the registered LeaveGuard */
  onViewChange: (view: SettingsView) => void;
  /** Guarded by the registered LeaveGuard */
  onClose: () => void;
  registerLeaveGuard: (guard: LeaveGuard | null) => void;
}

// A dialog, sheet or menu open on top of the page owns Escape.
const OVERLAY_SELECTOR = '.modal-overlay, .update-modal-overlay, .changelog-modal-overlay, [aria-modal="true"]';

export function SettingsPage({
  view,
  exiting,
  onViewChange,
  onClose,
  registerLeaveGuard,
}: SettingsPageProps) {
  const { reduced } = useMotion();
  const [entering, setEntering] = useState(!reduced);
  const [editorOpen, setEditorOpen] = useState(false);
  const activeNavRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!entering) return;
    const id = setTimeout(() => setEntering(false), UI.SETTINGS_PAGE.ENTER_DELAY_MS + UI.SETTINGS_PAGE.ENTER_MS);
    return () => clearTimeout(id);
  }, [entering]);

  useEffect(() => {
    activeNavRef.current?.focus({ preventScroll: true });
  }, []);

  useEffect(() => {
    if (exiting) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.defaultPrevented) return;
      if (document.querySelector(OVERLAY_SELECTOR)) return;
      event.preventDefault();
      onClose();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [exiting, onClose]);

  const renderSection = () => {
    switch (view) {
      case 'general':
        return <GeneralSection />;
      case 'writing':
        return <WritingSection />;
      case 'profiles':
        return <ProfilesSection registerLeaveGuard={registerLeaveGuard} onEditorChange={setEditorOpen} />;
      case 'downloads':
        return <DownloadsSection />;
      case 'about':
        return <AboutSection />;
    }
  };

  const phase = exiting ? ' is-exiting' : entering ? ' is-entering' : '';

  return (
    <div className={`settings-page${phase}`} inert={exiting || undefined} aria-hidden={exiting || undefined}>
      <SettingsNav view={view} onSelect={onViewChange} activeRef={activeNavRef} />
      <div className="settings-page__island settings-page__main">
        <div key={view} className={`settings-page__scroll${editorOpen ? ' settings-page__scroll--editor' : ''}`}>
          {renderSection()}
        </div>
      </div>
    </div>
  );
}
