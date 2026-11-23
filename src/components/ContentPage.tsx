import React from 'react';
import { BookOpen, Grid, Sprout } from 'lucide-react';
import type { t } from '../i18n/translations';
import type { Language } from '../types/guild';

export const CONTENT_LANGUAGES = [
  { code: 'nb', name: 'Norsk', short: 'NO' },
  { code: 'en', name: 'English', short: 'EN' },
  { code: 'de', name: 'Deutsch', short: 'DE' },
] as const;

export const CONTENT_LABEL_KEYS = [
  'appTitle', 'navPlanner', 'navGarden', 'navGuides', 'photoCreditsTitle',
  'footerText', 'impressumLink', 'appPrivacyLink', 'loadingFallback',
] as const;
export type ContentLabels = Pick<ReturnType<typeof t>, typeof CONTENT_LABEL_KEYS[number]>;

export function contentPath(route: string, language: Language, defaultLanguage: Language) {
  if (language === defaultLanguage) return route;
  return route.replace(/^\/(guides|credits)\//, `/$1/${language}/`);
}

interface ContentPageProps {
  route: string;
  language: Language;
  defaultLanguage: Language;
  legalRoutes: string[];
  labels: ContentLabels;
  guideAnchors?: Record<string, string[]>;
  children: React.ReactNode;
}

export function ContentPage({ route, language, defaultLanguage, legalRoutes, labels: tr, guideAnchors, children }: ContentPageProps) {
  const planner = `/?lang=${language}`;
  const garden = `/garten/?lang=${language}`;
  const guides = contentPath('/guides/', language, defaultLanguage);
  const credits = contentPath('/credits/', language, defaultLanguage);
  const section = guideAnchors ? 'guides' : 'credits';

  return (
    <div id="content-page" className="min-h-screen bg-stone-100 flex flex-col font-sans text-stone-900"
      data-route={route} data-default-language={defaultLanguage}
      data-guide-anchors={guideAnchors && JSON.stringify(guideAnchors)}>
      <header className="sticky top-0 z-40 bg-stone-100/85 backdrop-blur-md">
        <div className="max-w-7xl mx-auto px-3 sm:px-6 lg:px-8 min-h-[72px] py-2 flex items-center flex-wrap justify-between gap-2 sm:gap-4">
          <a href={planner} className="flex items-center gap-3 shrink-0" aria-label={tr.appTitle}>
            <div className="w-9 h-9 rounded-xl bg-forest-700 text-cream flex items-center justify-center">
              <Sprout className="w-5 h-5" strokeWidth={1.75} />
            </div>
            <span className="hidden md:block text-xl font-semibold">pflanzengilde<span className="text-forest-600">.de</span></span>
          </a>
          <nav className="flex items-center bg-white/70 p-1 rounded-xl text-sm mr-auto ring-1 ring-stone-900/5">
            {[
              { href: planner, text: tr.navPlanner, Icon: Sprout },
              { href: garden, text: tr.navGarden, Icon: Grid },
              { href: guides, text: tr.navGuides, Icon: BookOpen },
            ].map(({ href, text, Icon }) => (
              <a key={href} href={href} aria-label={text}
                aria-current={section === 'guides' && href === guides ? 'location' : undefined}
                className="h-8 px-2 sm:px-3 rounded-lg font-medium text-stone-600 hover:bg-stone-900/5 aria-[current=location]:bg-stone-900 aria-[current=location]:text-cream flex items-center gap-2">
                <Icon className="w-4 h-4 shrink-0" strokeWidth={1.75} />
                <span className="hidden sm:inline">{text}</span>
              </a>
            ))}
          </nav>
          <div className="flex items-center gap-1 shrink-0">
            {CONTENT_LANGUAGES.map(({ code, name, short }) => {
              const target = contentPath(route, code, defaultLanguage);
              return (
                <a key={code}
                  href={target === route ? `${target}?lang=${code}` : target}
                  hrefLang={code} lang={code} aria-label={name}
                  aria-current={code === language ? 'true' : undefined}
                  data-language-link
                  className="px-2 py-1 rounded-lg text-xs font-bold text-stone-600 hover:bg-white aria-[current=true]:bg-stone-900 aria-[current=true]:text-cream">
                  {short}
                </a>
              );
            })}
          </div>
        </div>
      </header>
      {children}
      <footer className="mt-16 pb-8 px-3 sm:px-6 lg:px-8 text-xs text-stone-500">
        <div className="max-w-7xl mx-auto rounded-[28px] bg-white px-6 py-6 sm:px-8 flex flex-col sm:flex-row items-center justify-between gap-4 text-center sm:text-left">
          <p>{tr.footerText}</p>
          <div className="flex flex-wrap items-center justify-center gap-x-4 gap-y-1">
            {[
              [planner, tr.navPlanner],
              [garden, tr.navGarden],
              [guides, tr.navGuides],
              [credits, tr.photoCreditsTitle],
              ...legalRoutes.map(page => [`/${page}/?lang=${language}`, page === 'impressum' ? tr.impressumLink : tr.appPrivacyLink]),
            ].map(([href, text]) => <a key={href} href={href} className="hover:text-stone-900 font-medium">{text}</a>)}
          </div>
        </div>
      </footer>
    </div>
  );
}
