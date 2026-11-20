import fs from 'fs';
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
const routes = [...legalRoutes, 'guides', 'credits', 'embed', 'garten', 'garden'];
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
    // Each page names itself as canonical; with the root's canonical Google treats /guides/ etc. as duplicates of
    // the home page. The embed widget keeps the root's (it is the same planner inside an iframe).
    const html = route === 'embed' ? baseHtml : setCanonical(baseHtml, `https://pflanzengilde.de/${route}/`);
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

// Per-tree share pages: crawlers read the meta tags, browsers get forwarded to the planner
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

  const clientRedirectScript = `
    <script>
      (function() {
        var isCrawler = /bot|googlebot|crawler|spider|crawling|discord|whatsapp|telegram|facebookexternalhit|twitterbot|slackbot/i.test(navigator.userAgent);
        if (!isCrawler) {
          var params = new URLSearchParams(window.location.search);
          if (!params.has('tree') && !params.has('g')) {
            params.set('tree', '${tree.id}');
          }
          window.location.replace('/?' + params.toString());
        }
      })();
    </script>
  `;

  treeHtml = treeHtml.replace('</head>', () => `${clientRedirectScript}\n</head>`);

  fs.writeFileSync(path.resolve(treeDir, 'index.html'), treeHtml, 'utf8');
}

console.log(`Pre-rendered ${STAR_TREES.length} share pages in dist/share/<tree-id>/index.html`);
