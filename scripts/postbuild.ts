import fs from 'fs';
import { createHash } from 'node:crypto';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { Guides, GUIDE_TABS, GUIDE_ANCHORS, guideTabPath } from '../src/components/Guides';
import { PhotoCredits } from '../src/components/PhotoCredits';
import { StaticContent } from '../src/components/StaticContent';
import { ContentPage, CONTENT_LABEL_KEYS, CONTENT_LANGUAGES, contentPath } from '../src/components/ContentPage';
import { t } from '../src/i18n/translations';
import path from 'path';
import { fileURLToPath } from 'url';
import { STAR_TREES } from '../src/data/starTrees';
import { getLoc, type Language } from '../src/types/guild';

// Language of the pre-rendered meta tags: SITE_LANG=de|en|nb; this Norwegian fork defaults to nb
const SITE_LANG: Language = process.env.SITE_LANG === 'de' || process.env.SITE_LANG === 'en' ? process.env.SITE_LANG : 'nb';
const META: Record<Language, { gardenTitle: string; gardenOgTitle: string; gardenDesc: string; guildSuffix: string }> = {
  de: {
    gardenTitle: 'Permakultur-Gartenplan (2D-Raster) • Pflanzengilde.de',
    gardenOgTitle: 'Permakultur-Gartenplan • 2D-Mehrbaum-Pflanzraster',
    gardenDesc: 'Interaktiver 2D-Permakultur-Gartenplan mit Leitbäumen, maßstabsgetreuen Pflanzabständen in Metern, geteilten Begleitpflanzen und Einkaufsliste auf Pflanzengilde.de.',
    guildSuffix: 'Permakultur-Gilde',
  },
  en: {
    gardenTitle: 'Permaculture garden plan (2D grid) • Pflanzengilde.de',
    gardenOgTitle: 'Permaculture garden plan • 2D multi-tree planting grid',
    gardenDesc: 'Interactive 2D permaculture garden plan with star trees, true-to-scale spacing in metres, shared companion plants and a shopping list on Pflanzengilde.de.',
    guildSuffix: 'permaculture guild',
  },
  nb: {
    gardenTitle: 'Hageplan for permakultur (2D-rutenett) • Pflanzengilde.de',
    gardenOgTitle: 'Hageplan for permakultur • 2D-rutenett med flere trær',
    gardenDesc: 'Interaktiv 2D-hageplan for permakultur med hovedtrær, planteavstander i riktig målestokk, delte følgeplanter og planteliste på Pflanzengilde.de.',
    guildSuffix: 'permakulturlaug',
  },
};
const meta = META[SITE_LANG];

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const distDir = path.resolve(__dirname, '../dist');
const publicDir = path.resolve(__dirname, '../public');

if (!fs.existsSync(path.resolve(distDir, 'index.html'))) {
  console.error('dist/index.html not found! Build may have failed.');
  process.exit(1);
}

const baseHtml = fs.readFileSync(path.resolve(distDir, 'index.html'), 'utf8');

// Function replacers so `$` sequences in content are not treated as replacement patterns.
const setTitle = (html: string, title: string) =>
  html.replace(/<title>.*?<\/title>/, () => `<title>${title}</title>`);

const setCanonical = (html: string, href: string) =>
  html.replace(/<link rel="canonical" href=".*?" \/>/, () => `<link rel="canonical" href="${href}" />`);

const setMeta = (html: string, attr: 'name' | 'property', key: string, content: string) =>
  html.replace(
    new RegExp(`<meta ${attr}="${key}" content=".*?" />`),
    () => `<meta ${attr}="${key}" content="${content}" />`
  );

// Static route copies so nginx serves index.html with the right meta tags per route
const legalRoutes = [
  { route: 'impressum', file: 'Impressum.tsx' },
  { route: 'datenschutz', file: 'Datenschutz.tsx' },
]
  .filter(({ file }) => fs.existsSync(path.resolve(__dirname, '../src/legal', file)))
  .map(({ route }) => route);
const legalLabels = JSON.stringify(Object.fromEntries(CONTENT_LANGUAGES.map(({ code }) => {
  const labels = t(code);
  return [code, Object.fromEntries(CONTENT_LABEL_KEYS.map(key => [key, labels[key]]))];
}))).replace(/</g, '\\u003c');
const routes = [...legalRoutes, 'embed', 'garten', 'garden'];
for (const route of routes) {
  const dir = path.resolve(distDir, route);
  fs.mkdirSync(dir, { recursive: true });

  if (route === 'garten' || route === 'garden') {
    const gardenTitle = meta.gardenTitle;
    const gardenOgTitle = meta.gardenOgTitle;
    const gardenDesc = meta.gardenDesc;
    const gardenOgImage = 'https://pflanzengilde.de/og/garden-plan.jpg';

    let gardenHtml = setTitle(baseHtml, gardenTitle);
    gardenHtml = setMeta(gardenHtml, 'name', 'description', gardenDesc);
    gardenHtml = setCanonical(gardenHtml, `https://pflanzengilde.de/${route}/`);
    gardenHtml = setMeta(gardenHtml, 'property', 'og:title', gardenOgTitle);
    gardenHtml = setMeta(gardenHtml, 'property', 'og:description', gardenDesc);
    // No static og:url, so Discord/Telegram cache per ?garden= URL instead of collapsing to the root
    gardenHtml = gardenHtml.replace(/<meta property="og:url" content=".*?" \/>\s*/, '');
    gardenHtml = setMeta(gardenHtml, 'property', 'og:image', gardenOgImage);
    gardenHtml = setMeta(gardenHtml, 'property', 'og:image:secure_url', gardenOgImage);
    gardenHtml = setMeta(gardenHtml, 'property', 'og:image:alt', gardenOgTitle);
    gardenHtml = setMeta(gardenHtml, 'name', 'twitter:title', gardenOgTitle);
    gardenHtml = setMeta(gardenHtml, 'name', 'twitter:description', gardenDesc);
    gardenHtml = setMeta(gardenHtml, 'name', 'twitter:image', gardenOgImage);

    fs.writeFileSync(path.resolve(dir, 'index.html'), gardenHtml, 'utf8');
    console.log(`Pre-rendered garden page dist/${route}/index.html`);
  } else {
    const html = route === 'embed'
      ? baseHtml
      : setCanonical(baseHtml, `https://pflanzengilde.de/${route}/`)
        .replace(/<html lang="[^"]*">/, `<html lang="${SITE_LANG}">`)
        .replace('</head>', () => `  <script type="application/json" id="legal-labels">${legalLabels}</script>\n  </head>`);
    fs.writeFileSync(path.resolve(dir, 'index.html'), html, 'utf8');
    console.log(`Mirrored dist/index.html to dist/${route}/index.html`);
  }
}

const publicOgDir = path.resolve(publicDir, 'og');
const distOgDir = path.resolve(distDir, 'og');
if (fs.existsSync(publicOgDir)) {
  fs.mkdirSync(distOgDir, { recursive: true });
  const ogFiles = fs.readdirSync(publicOgDir);
  for (const file of ogFiles) {
    fs.copyFileSync(path.join(publicOgDir, file), path.join(distOgDir, file));
  }
  console.log(`Mirrored ${ogFiles.length} OG images to dist/og/`);
}

// Per-tree share pages keep their own social metadata and open the planner at the same URL.
const escapeAttr = (s: string) => s.replace(/"/g, '&quot;');

for (const tree of STAR_TREES) {
  const treeDir = path.resolve(distDir, `share/${tree.id}`);
  fs.mkdirSync(treeDir, { recursive: true });

  const treeName = getLoc(tree.commonName, SITE_LANG);
  const treeDescription = getLoc(tree.description, SITE_LANG);
  const treeTitle = `${treeName} • Pflanzengilde.de`;
  const treeOgTitle = SITE_LANG === 'de' ? `${treeName} ${meta.guildSuffix}` : `${treeName}: ${meta.guildSuffix}`;
  const treeDesc = escapeAttr(treeDescription);
  // Truncate before escaping so an entity is never cut in half
  const treeMetaDesc = `${escapeAttr(treeDescription.substring(0, 160))}...`;
  const ogImageUrl = `https://pflanzengilde.de/og/${tree.id}.jpg`;
  const sharePageUrl = `https://pflanzengilde.de/share/${tree.id}/`;

  let treeHtml = setTitle(baseHtml, treeTitle);
  treeHtml = setMeta(treeHtml, 'name', 'description', treeMetaDesc);
  treeHtml = setCanonical(treeHtml, sharePageUrl);
  treeHtml = setMeta(treeHtml, 'property', 'og:title', treeOgTitle);
  treeHtml = setMeta(treeHtml, 'property', 'og:description', treeDesc);
  treeHtml = setMeta(treeHtml, 'property', 'og:url', sharePageUrl);
  treeHtml = setMeta(treeHtml, 'property', 'og:image', ogImageUrl);
  treeHtml = setMeta(treeHtml, 'property', 'og:image:secure_url', ogImageUrl);
  treeHtml = setMeta(treeHtml, 'name', 'twitter:title', treeOgTitle);
  treeHtml = setMeta(treeHtml, 'name', 'twitter:description', treeDesc);
  treeHtml = setMeta(treeHtml, 'name', 'twitter:image', ogImageUrl);

  fs.writeFileSync(path.resolve(treeDir, 'index.html'), treeHtml, 'utf8');
  const legacyTreeDir = path.resolve(distDir, `guild/${tree.id}`);
  fs.mkdirSync(legacyTreeDir, { recursive: true });
  fs.writeFileSync(path.resolve(legacyTreeDir, 'index.html'), treeHtml, 'utf8');
}

console.log(`Pre-rendered ${STAR_TREES.length} share pages in dist/share/<tree-id>/index.html`);


// Publish all three languages as complete documents. The small script handles
// older query links and the documented-pair filter without mounting React.
const client = fs.readFileSync(path.join(__dirname, 'content-pages.js'));
const clientName = `content-${createHash('sha256').update(client).digest('hex').slice(0, 16)}.js`;
fs.writeFileSync(path.join(distDir, 'assets', clientName), client);
const contentShell = baseHtml
  .replace(/\s*<script\b[^>]*type="module"[^>]*>[\s\S]*?<\/script>/gi, '')
  .replace(/\s*<link\b[^>]*rel="modulepreload"[^>]*>/gi, '');
const guideRoutes = new Set(GUIDE_TABS.map(({ id }) => guideTabPath(id)));

for (const { code: language } of CONTENT_LANGUAGES) {
  const text = t(language);
  for (const { route, title, description, page, guide } of [
    ...GUIDE_TABS.map(({ id, label }) => ({
      route: guideTabPath(id), title: text[label], description: text.guidesSubtitle, guide: true,
      page: createElement(Guides, { language, initialTab: id }),
    })),
    { route: '/credits/', title: text.photoCreditsTitle, description: text.photoCreditsIntro, guide: false,
      page: createElement(PhotoCredits, { language }) },
  ]) {
    const destination = contentPath(route, language, SITE_LANG);
    const canonical = `https://pflanzengilde.de${destination}`;
    const dir = path.join(distDir, destination);
    fs.mkdirSync(dir, { recursive: true });
    const markup = renderToStaticMarkup(createElement(StaticContent.Provider, { value: true },
      createElement(ContentPage, {
        route, language, defaultLanguage: SITE_LANG, legalRoutes, labels: text, guideAnchors: guide ? GUIDE_ANCHORS : undefined,
        children: page,
      }))).replace(/href="(\/guides\/[^"#?]*)(#[^"]*)?"/g, (attribute, target, hash = '') =>
        guideRoutes.has(target) ? `href="${contentPath(target, language, SITE_LANG)}${hash}"` : attribute
      );
    const pageTitle = escapeAttr(`${title} • Pflanzengilde.de`);
    let html = contentShell.replace(/<html lang="[^"]*">/, `<html lang="${language}">`);
    html = setTitle(html, pageTitle);
    html = setCanonical(html, canonical);
    html = setMeta(html, 'name', 'description', escapeAttr(description));
    html = setMeta(html, 'property', 'og:url', canonical);
    html = setMeta(html, 'property', 'og:title', pageTitle);
    html = setMeta(html, 'property', 'og:description', escapeAttr(description));
    html = setMeta(html, 'property', 'og:locale', { nb: 'nb_NO', de: 'de_DE', en: 'en_US' }[language]);
    html = setMeta(html, 'name', 'twitter:title', pageTitle);
    html = setMeta(html, 'name', 'twitter:description', escapeAttr(description));
    const alternateLinks = CONTENT_LANGUAGES.map(({ code }) =>
      `<link rel="alternate" hreflang="${code}" href="https://pflanzengilde.de${contentPath(route, code, SITE_LANG)}">`
    );
    alternateLinks.push(`<link rel="alternate" hreflang="x-default" href="https://pflanzengilde.de${route}">`);
    html = html.replace('</head>', () =>
      `  ${alternateLinks.join('\n    ')}\n    <script type="module" src="/assets/${clientName}"></script>\n  </head>`);
    html = html.replace('<div id="root"></div>', () => markup);
    fs.writeFileSync(path.join(dir, 'index.html'), html, 'utf8');
  }
}

console.log(`Rendered ${(GUIDE_TABS.length + 1) * CONTENT_LANGUAGES.length} localized guide and photo-credit pages to HTML`);
