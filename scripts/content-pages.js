const content = document.getElementById('content-page');
const languageKey = 'permaculture_plant_guild_lang';
const normalize = value => {
  const code = value?.toLowerCase().split('-')[0];
  if (code === 'no' || code === 'nn' || code === 'nb') return 'nb';
  if (code === 'en' || code === 'de') return code;
};

function restoreLink() {
  const { pathname, search, hash } = location;
  const params = new URLSearchParams(search);
  const renderedLanguage = document.documentElement.lang;
  const { route, defaultLanguage, guideAnchors } = content.dataset;
  const localizedPath = pathname.replace(/\/index\.html$/, '').replace(/\/$/, '') !== route.replace(/\/$/, '');
  const explicit = normalize(params.get('lang'));
  let saved;
  try { saved = normalize(localStorage.getItem(languageKey)); } catch {}
  const preferred = explicit ?? (localizedPath ? renderedLanguage : saved ?? normalize(navigator.language));
  const language = preferred ?? defaultLanguage;
  let destination = route;

  if (guideAnchors) {
    const anchors = JSON.parse(guideAnchors);
    const legacy = params.get('tab');
    const anchor = hash.slice(1);
    const anchorTab = Object.keys(anchors).find(id =>
      id === anchor || anchors[id].some(prefix => anchor.startsWith(prefix)));
    const legacyTab = Object.hasOwn(anchors, legacy) ? legacy : undefined;
    const tab = legacyTab ?? (route === '/guides/' ? anchorTab : undefined);
    if (tab) destination = tab === 'roles' ? '/guides/' : `/guides/${tab}/`;
    // An explicit roles tab also overrides anchors that point to another guide.
    if (legacyTab && (tab !== 'roles' || !anchorTab || anchorTab === 'roles')) params.delete('tab');
  }
  if (language !== defaultLanguage) destination = destination.replace(/^\/(guides|credits)\//, `/$1/${language}/`);
  if (explicit || localizedPath) {
    try { localStorage.setItem(languageKey, language); } catch {}
  }
  if (pathname.replace(/\/index\.html$/, '/').replace(/\/?$/, '/') !== destination || params.toString() !== new URLSearchParams(search).toString()) {
    const query = params.toString();
    location.replace(`${destination}${query ? `?${query}` : ''}${hash}`);
    return;
  }
  for (const link of content.querySelectorAll('[data-language-link]')) link.hash = hash;
}

function enablePairFilters(section) {
  const limit = Number(section.dataset.coverPairs);
  const rows = [...section.querySelectorAll('[data-pair-verdict]')];
  const filters = [...section.querySelectorAll('[data-pair-filter]')];
  const more = section.querySelector('[data-pair-more]');
  let filter = 'ALL';
  let expanded = false;

  function render() {
    let count = 0;
    for (const row of rows) {
      const matches = filter === 'ALL' || row.dataset.pairVerdict === filter;
      if (matches) count++;
      row.hidden = !matches || (!expanded && count > limit);
    }
    for (const button of filters) button.setAttribute('aria-pressed', String(button.dataset.pairFilter === filter));
    if (more) {
      more.hidden = count <= limit;
      more.setAttribute('aria-expanded', String(expanded));
      more.querySelector('span').textContent = expanded ? more.dataset.pairFewer : `${more.dataset.pairMore} (${count})`;
    }
  }

  for (const button of filters) {
    button.hidden = false;
    button.addEventListener('click', () => {
      filter = button.dataset.pairFilter;
      expanded = false;
      render();
    });
  }
  more?.addEventListener('click', () => {
    expanded = !expanded;
    render();
  });
  render();
}

restoreLink();
window.addEventListener('hashchange', restoreLink);
for (const section of content.querySelectorAll('[data-cover-pairs]')) enablePairFilters(section);
for (const img of content.querySelectorAll('[data-static-thumbnail]')) {
  if (img.complete && !img.naturalWidth) img.hidden = true;
  else img.addEventListener('error', () => { img.hidden = true; }, { once: true });
}
