import init, { mount } from './planner.js';

const root = document.getElementById('planner');
const query = new URLSearchParams(location.search);
const preloaded = new Map();
function preload(name) {
  preloaded.set(name, fetch(`${root.dataset.assets}/${name}`)
    .then(response => response.ok ? response.text() : null)
    .catch(() => null));
}

function normalizeLanguage(language) {
  const code = language?.toLowerCase().split('-')[0];
  if (code === 'en' || code === 'de' || code === 'nb') return code;
  if (code === 'no' || code === 'nn') return 'nb';
}

function guessLanguage() {
  const explicit = normalizeLanguage(query.get('lang'));
  if (explicit) return explicit;
  // Older codes can contain the language. Rust resolves it before choosing the catalogue.
  if (query.has('g') || query.has('garden')) return;
  let saved;
  try { saved = localStorage.getItem('permaculture_plant_guild_lang'); } catch {}
  return normalizeLanguage(saved) ?? normalizeLanguage(navigator.language) ?? 'nb';
}

const language = guessLanguage();
if (language) preload(`catalog-${language}.json`);
if (/^\/(?:rust\/)?(?:garten|garden)(?:\/index\.html)?\/?$/.test(location.pathname) || query.has('garden')) {
  preload('garden-tree-data.json');
}

init().then(() => mount(preloaded)).catch(error => {
  console.error(error);
  const messages = {
    nb: 'Kunne ikke starte planteplanleggeren. Last siden på nytt.',
    en: 'The planting planner could not start. Reload the page.',
    de: 'Der Pflanzplaner konnte nicht starten. Lade die Seite neu.',
  };
  const message = document.createElement('p');
  message.setAttribute('role', 'alert');
  message.textContent = messages[document.documentElement.lang] ?? messages.nb;
  root.querySelector('main')?.prepend(message);
});
