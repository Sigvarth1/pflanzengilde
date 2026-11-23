import React, { Suspense, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ContentPage, type ContentLabels } from './components/ContentPage';
import { Datenschutz, Impressum } from './legal-pages';
import type { Language } from './types/guild';

const languageKey = 'permaculture_plant_guild_lang';
const pages = { impressum: Impressum, datenschutz: Datenschutz };

function supportedLanguage(value: string | null | undefined): Language | undefined {
  const lang = value?.toLowerCase().split('-')[0];
  if (lang === 'nb' || lang === 'en' || lang === 'de') return lang;
  if (lang === 'no' || lang === 'nn') return 'nb';
}

const defaultLanguage = supportedLanguage(document.documentElement.lang) ?? 'nb';

function readLanguage(): Language {
  const explicit = supportedLanguage(new URLSearchParams(location.search).get('lang'));
  if (explicit) return explicit;
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(languageKey);
  } catch {
    // Storage can be disabled in embedded or private contexts.
  }
  return supportedLanguage(saved) ?? supportedLanguage(navigator.language) ?? defaultLanguage;
}

const route = location.pathname.split('/')[1];
const Page = Object.entries(pages).find(([name]) => name === route)?.[1];
const legalRoutes = Object.entries(pages).filter(([, page]) => page !== null).map(([name]) => name);

function LegalPage({ labels }: { labels: Record<Language, ContentLabels> }) {
  const [language, setLanguage] = useState(readLanguage);
  useEffect(() => {
    document.documentElement.lang = language;
    try {
      localStorage.setItem(languageKey, language);
    } catch {
      // The current language still works when storage is disabled.
    }
  }, [language]);
  if (!Page) return null;
  const text = labels[language];
  const navigate = (href: string) => {
    const target = new URL(href, location.href);
    if (target.origin === location.origin && !target.searchParams.has('lang')) {
      target.searchParams.set('lang', language);
    }
    location.assign(target);
  };
  return (
    <ContentPage route={`/${route}/`} language={language} defaultLanguage={defaultLanguage} legalRoutes={legalRoutes} labels={text}>
      <Suspense fallback={<p className="mx-auto px-4 py-12">{text.loadingFallback}</p>}>
        <Page language={language} setLanguage={setLanguage} onBack={() => navigate('/')} onNavigate={navigate} />
      </Suspense>
    </ContentPage>
  );
}

const root = document.getElementById('root');
const localeData = document.getElementById('legal-labels')?.textContent;
if (root && Page && localeData) {
  const labels = JSON.parse(localeData) as Record<Language, ContentLabels>;
  createRoot(root).render(<LegalPage labels={labels} />);
}
