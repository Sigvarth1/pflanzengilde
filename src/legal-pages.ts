import { lazy, type ComponentType } from 'react';
import type { Language } from './types/guild';

interface LegalPageProps {
  language: Language;
  setLanguage: (lang: Language) => void;
  onBack: () => void;
  onNavigate: (path: string) => void;
}

// Operators can provide named exports under src/legal/ without editing the site.
const modules = import.meta.glob<Record<string, ComponentType<LegalPageProps>>>('./legal/*.tsx');

function loadLegalPage(name: string) {
  const load = modules[`./legal/${name}.tsx`];
  return load ? lazy(() => load().then(module => ({ default: module[name] }))) : null;
}

export const Impressum = loadLegalPage('Impressum');
export const Datenschutz = loadLegalPage('Datenschutz');
