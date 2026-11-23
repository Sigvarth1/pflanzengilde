import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cp, mkdir, mkdtemp, readFile, readdir, rename, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

const root = new URL('../', import.meta.url);
const crate = new URL('rust/site/', root);
const dist = new URL('dist/', root);
const rust = new URL('rust/', dist);
const primary = process.argv.includes('--primary');
const buildDir = new URL('target/', crate);
const wasm = new URL('wasm32-unknown-unknown/release/pflanzengilde_site.wasm', buildDir);
const localBindgen = new URL('tools/wasm-bindgen', buildDir);
const bindgen = process.env.PLANTGUILD_WASM_BINDGEN || (
  await stat(localBindgen).then(file => file.isFile(), () => false)
    ? fileURLToPath(localBindgen)
    : 'wasm-bindgen'
);

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run('cargo', [
  'build', '--locked', '--release', '--target', 'wasm32-unknown-unknown',
  '--manifest-path', fileURLToPath(new URL('Cargo.toml', crate)),
  '--target-dir', fileURLToPath(buildDir),
]);
await mkdir(rust, { recursive: true });
const staging = await mkdtemp(fileURLToPath(new URL('.build-', rust)));
const output = pathToFileURL(staging + path.sep);
try {
run(bindgen, [
  '--target', 'web', '--no-typescript', '--out-dir', fileURLToPath(output),
  '--out-name', 'planner', fileURLToPath(wasm),
]);
await cp(new URL('web/planner.css', crate), new URL('planner.css', output));
await cp(new URL('web/bootstrap.js', crate), new URL('bootstrap.js', output));
for (const font of ['figtree-latin-wght-normal.woff2', 'figtree-latin-ext-wght-normal.woff2']) {
  await cp(new URL(`node_modules/@fontsource-variable/figtree/files/${font}`, root), new URL(font, output));
}
if (!primary) await cp(new URL('public/', root), dist, { recursive: true });

const vite = await createServer({
  root: fileURLToPath(root),
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: 'custom',
  optimizeDeps: { noDiscovery: true, include: [] },
});
try {
  const { STAR_TREES } = await vite.ssrLoadModule('/src/data/starTrees.ts');
  const { GUILD_PLANTS } = await vite.ssrLoadModule('/src/data/guildPlants.ts');
  const { getImageCredit } = await vite.ssrLoadModule('/src/data/imageCredits.ts');
  const { WORLD_LAND_PATH } = await vite.ssrLoadModule('/src/data/worldLandPath.ts');
  const { WORLD_REGIONS, CLIMATE_DISPLAY_BOUNDARIES } =
    await vite.ssrLoadModule('/src/data/worldClimateRegions.ts');
  const { getLoc, getLocList } = await vite.ssrLoadModule('/src/types/guild.ts');
  const { t, formatNumber, translateRole, translateLayer, translateSun, translateSector, translateJuglone, translateStarCategory, translateSeason } = await vite.ssrLoadModule('/src/i18n/translations.ts');
  const { DEFAULT_GUILD_PLANT_IDS, GUILD_PRESETS } = await vite.ssrLoadModule('/src/core/guildPresets.ts');
  const { PEST_HOST_CONFLICTS } = await vite.ssrLoadModule('/src/core/pestHostConflicts.ts');
  const { getPlantingSeasons, getHarvestSeasons, PHENO_SEASONS, analyzeSeasonalRoleGaps } = await vite.ssrLoadModule('/src/core/seasonalGapEngine.ts');
  const { CHOP_INSTRUCTIONS_MAP, CHOP_PLANT_ANCHORS, BLOOM_STAGE_CHOP_DATES } =
    await vite.ssrLoadModule('/src/core/calendarExporter.ts');
  const { getCompanionPestDefenseForTree, PEST_DEFENSE_RULES, citationHref } =
    await vite.ssrLoadModule('/src/core/pestCompanionEngine.ts');
  const { getGroundCoverSpec } = await vite.ssrLoadModule('/src/data/groundCoverSpecs.ts');
  const { coverOverlap } = await vite.ssrLoadModule('/src/data/groundCoverOverlap.ts');
  const conflictSources = await vite.ssrLoadModule('/src/core/antagonistEngine.ts');
  const placementFor = (plants, ordered) => plants.map((plant, i) => ({
    instanceId: plant.id, plantId: plant.id, plant, zone: plant.preferredZone,
    sector: plant.preferredSector, angleDeg: ordered ? i * 180 : 0,
    distanceM: ordered ? 3 + i : 1,
  }));
  // Extract the existing translated content. The Rust planner evaluates the rules against its own placements.
  const guildNoticeContent = new Map();
  const spacedPlants = ['plant-chives', 'plant-white-clover']
    .map(id => GUILD_PLANTS.find(plant => plant.id === id));
  const outerPlants = ['plant-fennel', 'plant-wormwood'].map(id => GUILD_PLANTS.find(plant => plant.id === id));
  for (const star of STAR_TREES) {
    for (const plants of [GUILD_PLANTS, spacedPlants, ...outerPlants.map(plant => [plant])]) {
      const report = conflictSources.analyzeGuildAntagonisms(star, plants, placementFor(plants, plants !== GUILD_PLANTS));
      for (const notice of [...report.conflicts, ...report.resolvedHarmonies]) {
        if (!guildNoticeContent.has(notice.id)) guildNoticeContent.set(notice.id, { star, notice });
      }
    }
  }
  const twinRoles = new Map();
  for (const plant of GUILD_PLANTS) {
    const name = plant.botanicalName.toLowerCase().trim();
    if (!plant.retired && !twinRoles.has(name)) twinRoles.set(name, plant.roles);
  }
  const groundCovers = GUILD_PLANTS.flatMap(plant => {
    const spec = getGroundCoverSpec(plant);
    if (!spec) return [];
    const {
      mode, rOuterFactor, rInnerExtraM, light, strategy, stolonRateMPerYr,
      edge, seasonLayer, coverFraction, denseMat, stripWidthM, clumpClearanceExtraM, drift,
    } = spec;
    return [{ plant, spec: {
      mode, rOuterFactor, rInnerExtraM, light, strategy, stolonRateMPerYr,
      edge, seasonLayer, coverFraction, denseMat, stripWidthM, clumpClearanceExtraM, drift,
    } }];
  });
  const competition = [];
  const groundCoverIds = new Set(groundCovers.map(({ plant }) => plant.id));
  for (let a = 0; a < groundCovers.length; a++) {
    for (let b = a + 1; b < groundCovers.length; b++) {
      const left = groundCovers[a].plant;
      const right = groundCovers[b].plant;
      const { verdict } = coverOverlap(left, right);
      if (verdict !== 'MOSAIC') competition.push([left.id, right.id, verdict]);
    }
  }
  await writeFile(new URL('ground-cover.json', output), JSON.stringify({
    species: Object.fromEntries(groundCovers.map(({ plant, spec }) => [plant.id, spec])),
    competition,
  }));
  const roles = ['NITROGEN_FIXER','DYNAMIC_ACCUMULATOR','POLLINATOR_MAGNET','PEST_REPELLER','LIVING_MULCH',
    'GRASS_BARRIER','ANTIFUNGAL','BIOMASS_PRODUCER','EDIBLE_UNDERSTORY'];
  const defenses = new Map(GUILD_PLANTS.map(plant => [plant.id,
    STAR_TREES.filter(tree => getCompanionPestDefenseForTree(plant, tree).length > 0).map(tree => tree.id),
  ]));
  const gapReasons = new Map();
  for (const plant of GUILD_PLANTS) {
    if (plant.retired) continue;
    for (const role of analyzeSeasonalRoleGaps([plant])) {
      for (const gap of role.gaps) gapReasons.set(`gap:${gap.role}:${gap.season.toLowerCase()}`, gap.reason);
    }
  }
  const keys = [
    'calendarTitle', 'calendarSubtitle', 'exportCalendar', 'exportCalendarSuccess', 'gardenPageCalendarError',
    'seasonEarlySpring', 'seasonLateSpring', 'seasonSummer', 'seasonAutumn', 'seasonWinter',
    'functionalRole', 'missingGap', 'gapsDetectedTitle', 'recommendedAllies', 'filterGapAction',
    'activeGapBannerTitle', 'clearGapFilter', 'noGapPlantsFound', 'seasonalCalendarAddToFillGap',
    'redundantSectionTitle', 'redundantSectionSub', 'redundantCoveredByLabel', 'redundantSwapForUncovered',
    'redundantRemoveAction', 'seasonalCalendarSwapWith',
    'calendarPlanting', 'calendarCatHarvest', 'bloom', 'noPlantChosenYet',
    'treeAgeLabel', 'treeAgeYoung', 'treeAgeEstablished', 'showGroundCovers', 'coverSeasonAll', 'coverSeasonLabel',
    'exportPdf', 'appPdfError', 'gardenPagePdfSuccess', 'gardenPagePdfError',
    'photoCreditsTitle', 'impressumLink', 'appPrivacyLink', 'shareEmbedCode',
    'shareGuildBtn', 'shareGuildDesc', 'shareGardenSubtitle', 'shareEmbedPreview', 'shareCopyLink',
    'shareLinkCopied', 'shareEmbedCodeCopied', 'shareGuildShareText', 'shareGardenShareText',
    'shareGuildShareDirectly', 'shareGuildShareViaDevice', 'shareGuildEmbedInstructions', 'shareGardenEmbedHint',
    'permacultureGuildLabel', 'embedCardCompanions', 'embedCardMore', 'embedCardTagline', 'openInPlanner',
    'shareGardenMultiGuildPlan', 'shareGardenStarTrees', 'shareGardenCompanions',
    'navPlanner', 'navGarden', 'navGuides', 'step1Title', 'step2Title',
    'coverageTitle', 'coverageSubtitle', 'coveredRatio', 'heroesTitle', 'heroesTip', 'rolesStacked', 'notYetCovered',
    'antagonistsSectionTitle', 'antagonistsSectionSub',
    'step2Subtitle', 'tabAll', 'tabRecommended', 'filterOptimalSoil', 'filterOptimalZone',
    'compatHiddenNoteGuild', 'compatShowHidden', 'compatHideHidden',
    'presetsTitle', 'presets', 'presetApple', 'presetWalnut', 'presetApricot', 'presetMinimal',
    'presetAppleSub', 'presetWalnutSub', 'presetApricotSub', 'presetMinimalSub', 'resetGuild',
    'soilType', 'soilLoam', 'soilClay', 'soilSandy', 'soilChalky', 'soilAcidic', 'soilSilt',
    'climateZone', 'zoneBoreal', 'zoneTemperate', 'zoneSubtropical', 'zoneTropical',
    'siteConditionsWorldMap', 'siteConditionsOpenWorldMap', 'climateZoneModalTitle', 'loadingFallback',
    'closeBtn',
    'spotHint', 'spotTitle', 'spotNothing', 'spotKindTrunk', 'spotKindCanopy', 'spotKindPlant',
    'spotKindCover', 'spotKindDrift', 'spotFromTrunk', 'groundCoverOutOfSeason',
    'toggleZones', 'toggleSun', 'spacingToggleCanopy', 'ladderPath', 'layer', 'lightPref', 'modalSector', 'modalJuglone',
    'modalRolesTitle', 'modalZone', 'modalSpreadHeight', 'modalDistance', 'modalSoilTitle',
    'plantDetailSuitable', 'plantDetailUnsuitable', 'modalPhenologyTitle', 'plantingTime', 'harvestTime',
    'plantingSeasonTag', 'harvestSeasonTag', 'bloomingTag', 'livingMulchTag', 'chopDropTag', 'pestDefenseTag',
    'dormantTag', 'addGuildBtn', 'removeGuildBtn', 'pestDefenseForStarTree',
    'keyPests', 'keyPestClickToFilter', 'activePestBannerTitle', 'clearPestFilter', 'noPestCompanionsFound',
    'evidenceLevelFieldProven', 'evidenceLevelScientific', 'evidenceSourcesTitle', 'evidenceBadgeNote',
    'effectiveCompanionsTitle', 'scientificMechanismTitle', 'evidenceLevelTooltip',
    'northHemisphere', 'southHemisphere', 'pdfPlantInventory', 'shareGuildTitle',
    'gardenGridTitle', 'gardenGridZoomIn', 'gardenGridZoomOut', 'gardenGridCenterView',
    'gardenGridPanTool', 'gardenGridSelectTool', 'gardenGridSelected',
    'gardenPageDefaultName', 'gardenPageInvalidJson', 'gardenPageImportedStars', 'gardenPageCleared', 'gardenSidebarActiveStarPlants', 'gardenSidebarAddStarPlant',
    'gardenSidebarNoStarPlants', 'gardenSidebarRemove', 'shareGardenTitle',
    'gardenPageSharedSource', 'gardenPagePlannerSource', 'gardenSidebarCompanions',
    'gardenPageShadePlantPlaced',
    'gardenPageCompanionNoTree', 'gardenPageCompanionAdded', 'gardenPageCompanionRemoved',
    'gardenPageCompanionLastOne', 'jugloneWarning',
    'gardenPagePendingPlaced', 'gardenPagePendingPlacedBeside', 'gardenPageHandoffError',
    'infraTitle', 'infraGardenShape', 'infraRaisedBeds', 'infraRect', 'infraCircle', 'infraLasso',
    'infraRectHint', 'infraCircleHint', 'infraLassoHint', 'infraDrawing', 'infraNoOutline', 'infraNoBeds', 'infraHeight', 'infraDelete', 'geoRefCancel',
    'gardenSidebarActiveCompanions', 'gardenSidebarNoCompanions',
    'exportJson', 'importPlan', 'importSuccess', 'importInvalidFormat',
    'pdfRadialMap', 'pdfRadialMapSub', 'zoneCollar', 'zoneBulb', 'zoneMid', 'zoneDrip', 'zoneOuter',
    'compatReasonJuglone', 'compatReasonEdaphic', 'compatReasonPestHost', 'compatAutoRemovedToast',
    'searchPlaceholder', 'noPlantsMatched', 'showAll', 'details', 'soilUnsuitable',
    'zoneIncompatibleBadge', 'footerText', 'soilBarTitle', 'soilBarSubtitle', 'soilOptimal', 'soilChipGood', 'soilChipBad', 'step1Subtitle', 'dripLine',
    'siteConditionsLoamPh', 'siteConditionsClayPh', 'siteConditionsSandyPh', 'siteConditionsChalkyPh',
    'siteConditionsAcidicPh', 'siteConditionsSiltPh',
  ];
  const photoCredit = (url, language) => {
    const credit = getImageCredit(url);
    return credit && t(language).photoCreditBadgeLabel
      .replace('{author}', credit.author).replace('{license}', credit.license);
  };
  const project = (species, language) => ({
    id: species.id,
    name: getLoc(species.commonName, language),
    names: ['nb', 'en', 'de'].map(lang => getLoc(species.commonName, lang)),
    botanicalName: species.botanicalName,
    description: species.retired ? '' : getLoc(species.description ?? species.notes, language),
    imageUrl: species.imageUrl,
    photoCredit: species.category ? photoCredit(species.imageUrl, language) : undefined,
    climateZones: species.climateZones,
    roles: species.roles?.map(role => translateRole(role, language)),
    unsuitableSoils: species.unsuitableSoils,
    suitableSoils: species.suitableSoils,
    preferredSoils: species.preferredSoils,
    categoryLabel: species.category ? translateStarCategory(species.category, language) : '',
    bloomLabel: species.bloomSeason ? translateSeason(species.bloomSeason, language) : '',
    dualRoles: species.category ? twinRoles.get(species.botanicalName.toLowerCase().trim()) : undefined,
    retired: species.retired,
    areaPlant: groundCoverIds.has(species.id) || undefined,
    color: species.color,
    matureRadiusM: species.matureRadiusM,
    heightM: species.matureHeightM ?? species.heightM,
    layerLabel: translateLayer(species.layer ?? 'CANOPY', language),
    spreadM: species.spreadM,
    zone: species.preferredZone,
    sector: species.preferredSector,
    sectorLabel: species.preferredSector ? translateSector(species.preferredSector, language) : '',
    layer: species.layer,
    roleIds: species.roles,
    minDistanceM: species.minDistanceM,
    maxDistanceM: species.maxDistanceM,
    sunPreference: species.sunPreference,
    sunLabel: species.sunPreference ? translateSun(species.sunPreference, language) : '',
    nitrogenFixer: species.category === 'NITROGEN_FIXING_TREE' || species.roles?.includes('NITROGEN_FIXER'),
    jugloneProducer: species.jugloneProducer,
    recommendedCompanions: species.recommendedCompanions,
    recommendedForTrees: species.recommendedForTrees,
    pestDefenseFor: defenses.get(species.id),
    activity: {
      active: species.seasonalActivity?.activeSeasons,
      foliage: species.seasonalActivity?.foliageSeasons,
      pestDeterrence: species.seasonalActivity?.pestDeterrenceSeasons,
      flowering: species.seasonalActivity?.floweringSeasons ?? (species.bloomSeason ? [species.bloomSeason] : []),
      planting: species.seasonalActivity ? getPlantingSeasons(species) : [species.plantingTime?.de?.toLowerCase().includes('herbst') ? 'AUTUMN' : 'EARLY_SPRING'],
      chop: species.seasonalActivity?.chopAndDropSeasons,
      harvest: species.seasonalActivity ? getHarvestSeasons(species) : (species.harvestSeason ? [species.harvestSeason] : []),
      plantingTime: species.plantingTime ? getLoc(species.plantingTime, language) : '',
      harvestTime: species.harvestTime ? getLoc(species.harvestTime, language) : '',
      soilNotes: species.soilNotes || species.soilAdvice ? getLoc(species.soilNotes ?? species.soilAdvice, language) : '',
      perennial: species.perennial,
      autumnPlanting: species.plantingTime?.de?.toLowerCase().includes('herbst') ?? false,
      treeCategory: species.category,
    },
    jugloneSensitive: species.jugloneTolerance === 'SENSITIVE',
    jugloneLabel: species.jugloneTolerance ? translateJuglone(species.jugloneTolerance, language) : '',
    vulnerabilities: species.vulnerabilities ? getLocList(species.vulnerabilities, language) : undefined,
  });
  const plantSources = Object.fromEntries(GUILD_PLANTS.map(plant => [plant.id, plant]));
  await writeFile(new URL('garden-plant-data.json', output), JSON.stringify(plantSources));
  const treeSources = Object.fromEntries(STAR_TREES.map(tree => [tree.id, tree]));
  await writeFile(new URL('garden-tree-data.json', output), JSON.stringify(treeSources));
  for (const language of ['nb', 'en', 'de']) {
    const dictionary = t(language);
    const sources = (items = []) => items.map(label => {
      const match = label.match(/https?:\/\/\S+|\b10\.\d{4,9}\/\S+/i)?.[0];
      const url = match?.replace(/[.,;]+$/, '');
      return { label, href: url?.startsWith('http') ? url : url && citationHref({ doi: url }) };
    });
    const notices = Object.fromEntries([...guildNoticeContent].map(([id, { star, notice }]) => {
      const tree = text => id.includes('nfixing-tree')
        ? getLoc(text, language).replaceAll(getLoc(star.commonName, language), '{tree}')
        : getLoc(text, language);
      let advice = id.startsWith('resolved-') ? '' : tree(notice.spatialAdvice);
      if (id === 'internal-walnut-sensitive-companions') {
        advice = advice.replace(formatNumber(1, 1, language), '{measured}');
      } else if (id === 'external-walnut-spatial-planning') {
        advice = advice.slice(advice.lastIndexOf('. ') + 2);
      } else if (['internal-allium-legume-proximity', 'internal-fennel-allelopathy', 'internal-wormwood-allelopathy'].includes(id)) {
        advice = advice.replace(formatNumber(0, 2, language), '{measured}').replace('0°', '{angle}°');
      }
      if (id === 'external-solanaceae-fruit-tree-verticillium') {
        advice = advice.replace(formatNumber(star.matureRadiusM + 2, 1, language), '{limit}');
      }
      return [id, {
        title: tree(notice.title), name: tree(notice.antagonistName), mechanism: tree(notice.mechanism),
        advice, sources: sources(notice.scientificCitations),
      }];
    }));
    await writeFile(new URL(`guild-notices-${language}.json`, output), JSON.stringify(notices));
    await writeFile(new URL(`plant-details-${language}.json`, output), JSON.stringify(Object.fromEntries(
      GUILD_PLANTS.map(plant => [plant.id, {
        credit: photoCredit(plant.imageUrl, language),
        sources: sources(plant.sources),
      }])
    )));
    const climateLabels = Object.fromEntries(Object.entries(dictionary).filter(([key]) =>
      /^(climateModal|climateZoneModalSubtitle|worldMapTitle|mapRegionHoverHint|sunPath|zone.*(?:Sub|Desc)$)/.test(key)));
    await writeFile(new URL(`climate-map-${language}.json`, output), JSON.stringify({
      land: WORLD_LAND_PATH,
      boundaries: CLIMATE_DISPLAY_BOUNDARIES.map(({ d, stroke, strokeWidth, dashArray, opacity }) =>
        ({ d, stroke, strokeWidth, dashArray, opacity })),
      regions: WORLD_REGIONS.map(region => {
        const suffix = region.id.toLowerCase().split('_').map(word => word[0].toUpperCase() + word.slice(1)).join('');
        return {
          ...region,
          name: getLoc(region.name, language),
          geoAreas: getLoc(region.geoAreas, language),
          latRange: getLoc(region.latRange, language),
          button: dictionary[`climateModalRegion${suffix}`],
        };
      }),
      labels: climateLabels,
    }));
    const data = {
      trees: STAR_TREES.map(tree => project(tree, language)),
      plants: GUILD_PLANTS.map(plant => project(plant, language)),
      defaultPlantIds: DEFAULT_GUILD_PLANT_IDS,
      presets: GUILD_PRESETS,
      pestDefenseRules: PEST_DEFENSE_RULES.map(rule => ({
        id: rule.id, title: getLoc(rule.ruleTitle, language), keywords: rule.keywords,
        treeIds: rule.starTreeIds, evidence: rule.evidence,
        companionIds: rule.companionPlantIds, companionEvidence: rule.companionEvidence,
        mechanism: getLoc(rule.scientificMechanism, language),
        companionRoles: Object.fromEntries(Object.entries(rule.companionRoles).map(([id, role]) => [id, getLoc(role, language)])),
        citations: rule.citations.map(citation => ({ label: citation.label, href: citationHref(citation) })),
      })),
      pestConflicts: PEST_HOST_CONFLICTS.filter(spec => spec.kind === 'INTERNAL').map(spec => ({
        id: spec.id, treeIds: spec.starTreeIds, hostIds: spec.hostPlantIds, hostStarIds: spec.hostStarIds ?? [],
        safeDistanceM: spec.safeDistanceM, severity: spec.severity, bedWall: spec.bedWall,
        title: getLoc(spec.title, language), mechanism: getLoc(spec.mechanism, language),
        citations: spec.scientificCitations,
      })),
      guildHosts: PEST_HOST_CONFLICTS.map(spec => ({
        id: spec.id, treeIds: spec.starTreeIds, hostIds: spec.hostPlantIds,
        external: spec.kind === 'EXTERNAL', severity: spec.severity, distance: spec.safeDistanceM,
      })),
      labels: Object.fromEntries([
        ...keys.map(key => [key, dictionary[key]]),
        ...Object.entries(dictionary).filter(([key]) => /^(cad|geoRef|navbarExport|pdf|gardenPdf|gardenPlan|gardenAntagonist|gardenResolve|gardenWarnings|gardenBedJuglone|gardenBedReduced|spacing|antagonist)/.test(key)),
      ]),
    };
    for (const [key, items] of Object.entries({
      juglone: [...new Set([...conflictSources.JUGLONE_MECHANISM_CITATIONS, ...conflictSources.JUGLONE_DISTANCE_CITATIONS])],
      allium: conflictSources.ALLIUM_LEGUME_CITATIONS,
      fennel: conflictSources.FENNEL_ALLELOPATHY_CITATIONS,
      wormwood: conflictSources.WORMWOOD_ALLELOPATHY_CITATIONS,
      soil: conflictSources.EDAPHIC_PH_CITATIONS,
    })) data.labels[`conflictSources:${key}`] = items.join('\n');
    for (const role of roles) data.labels[`role:${role}`] = translateRole(role, language);
    for (const [key, reason] of gapReasons) data.labels[key] = getLoc(reason, language);
    const seasonKeys = ['seasonEarlySpring', 'seasonLateSpring', 'seasonSummer', 'seasonAutumn', 'seasonWinter'];
    const northMonths = [[3, 4], [5, 6], [7, 8], [9, 10], [11, 2]];
    const monthName = new Intl.DateTimeFormat(language, { month: 'long', timeZone: 'UTC' });
    for (const [i, season] of PHENO_SEASONS.entries()) {
      data.labels[`months:${seasonKeys[i]}:north`] = getLoc(season.months, language);
      data.labels[`months:${seasonKeys[i]}:south`] = northMonths[i].map(month =>
        monthName.format(new Date(Date.UTC(2000, (month + 5) % 12, 1)))
      ).join(' – ');
    }
    const chop = Object.fromEntries(Object.entries(CHOP_INSTRUCTIONS_MAP).map(([id, instructions]) => [
      id, Object.fromEntries(['howToCut', 'howMuch', 'whereToSpread', 'nutrientBenefit']
        .map(key => [key, getLoc(instructions[key], language)])),
    ]));
    const calendarLabels = Object.fromEntries(Object.entries(dictionary)
      .filter(([key]) => key.startsWith('calendar') || key.startsWith('season')));
    await writeFile(new URL(`calendar-${language}.json`, output), JSON.stringify({
      labels: calendarLabels, chop, anchors: CHOP_PLANT_ANCHORS, overrides: BLOOM_STAGE_CHOP_DATES,
    }));
    await writeFile(new URL(`catalog-${language}.json`, output), JSON.stringify(data));
  }
} finally {
  await vite.close();
}

const hash = createHash('sha256');
for (const name of (await readdir(staging)).sort()) {
  hash.update(name).update('\0').update(await readFile(new URL(name, output)));
}
const revision = hash.digest('hex').slice(0, 16);
const assetDirectory = new URL(`${revision}/`, rust);
if (!(await stat(assetDirectory).then(entry => entry.isDirectory(), () => false))) {
  await rename(staging, fileURLToPath(assetDirectory));
}
const assets = `/rust/${revision}`;
const previewHtml = await readFile(new URL('web/index.html', crate), 'utf8');
const body = previewHtml.match(/<body>([\s\S]*?)<\/body>/)?.[1];
if (!body) throw new Error('The Rust entry template needs a body');
const legal = await Promise.all(
  ['impressum', 'datenschutz'].map(route =>
    stat(new URL(`${route}/index.html`, dist)).then(entry => entry.isFile() ? route : '', () => '')
  )
);
const stylesAndLoader = (head) => `${head}
    <link rel="preload" href="${assets}/figtree-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin>
    <link rel="stylesheet" href="${assets}/planner.css">
    <link rel="modulepreload" href="${assets}/planner.js">
    <link rel="preload" href="${assets}/planner_bg.wasm" as="fetch" type="application/wasm" crossorigin>
    <script type="module" src="${assets}/bootstrap.js"></script>`;
function entry(meta, preview) {
  const lang = meta.match(/<html lang="([a-z-]+)"/i)?.[1] ?? 'nb';
  const head = meta.match(/<head>([\s\S]*?)<\/head>/)?.[1];
  if (!head) throw new Error('The planner entry is missing its metadata');
  const metadata = head.replace(/\s*<script\b[^>]*\btype="module"[^>]*>[\s\S]*?<\/script>/gi, '')
    .replace(/\s*<link\b[^>]*\brel="(?:stylesheet|modulepreload)"[^>]*>/gi, '')
    .replace(/\s*<link\b[^>]*\bhref="\/rust\/[^"]*\/planner_bg\.wasm"[^>]*>/gi, '');
  const mount = body.replace('id="planner"', `id="planner" data-assets="${assets}" data-preview="${preview}" data-legal="${legal.filter(Boolean).join(' ')}"`);
  return `<!doctype html>\n<html lang="${lang}">\n<head>${stylesAndLoader(metadata)}\n</head>\n<body>${mount}</body>\n</html>\n`;
}
await writeFile(new URL('index.html', rust), entry(previewHtml, true));
for (const name of ['garten', 'garden']) {
  await mkdir(new URL(`${name}/`, rust), { recursive: true });
  await writeFile(new URL(`${name}/index.html`, rust), entry(previewHtml, true));
}
if (primary) {
  const share = new URL('share/', dist);
  const shared = (await readdir(share, { withFileTypes: true })).filter(file => file.isDirectory());
  const routes = ['index.html', 'garten/index.html', 'garden/index.html', 'embed/index.html',
    ...shared.flatMap(file => [`share/${file.name}/index.html`, `guild/${file.name}/index.html`])];
  // Validate and prepare every replacement before writing the route entry points.
  const entries = await Promise.all(routes.map(async (route) => {
    const file = new URL(route, dist);
    return [file, entry(await readFile(file, 'utf8'), false)];
  }));
  await Promise.all(entries.map(([file, html]) => writeFile(file, html)));
  console.log(`Rust now serves ${entries.length} planner entry points from ${assets}/`);
} else {
  console.log(`Built the Rust planner preview with assets at ${assets}/`);
}
} finally {
  await rm(staging, { recursive: true, force: true });
}
