import React, { useState } from 'react';
import { ClimateZone, Hemisphere, Language, SoilType, StarTree, TreeAgeMode, getLoc } from '../types/guild';
import { Mountain, AlertTriangle, CheckCircle2, Info, Droplets, Globe, Compass, Check } from 'lucide-react';
import { t, translateClimateZone } from '../i18n/translations';
import { ClimateZoneModal } from './ClimateZoneModal';
import { TreeAgeToggle } from './TreeAgeToggle';

interface SiteConditionsBarProps {
  language: Language;
  selectedSoil: SoilType;
  onSelectSoil: (soil: SoilType) => void;
  selectedTree: StarTree;
  selectedZone: ClimateZone;
  onSelectZone: (zone: ClimateZone) => void;
  hemisphere: Hemisphere;
  onSelectHemisphere: (hemisphere: Hemisphere) => void;
  treeAge?: TreeAgeMode;
  onSelectTreeAge?: (age: TreeAgeMode) => void;
}

export const SiteConditionsBar: React.FC<SiteConditionsBarProps> = ({
  language,
  selectedSoil,
  onSelectSoil,
  selectedTree,
  selectedZone,
  onSelectZone,
  hemisphere,
  onSelectHemisphere,
  treeAge,
  onSelectTreeAge,
}) => {
  const tr = t(language);
  const [isMapModalOpen, setIsMapModalOpen] = useState(false);

  const SOIL_OPTIONS: {
    id: SoilType;
    label: string;
    description: string;
    imageUrl: string;
    texture: string;
    drainage: string;
    ph: string;
  }[] = [
    {
      id: 'LOAM',
      label: tr.soilLoam,
      description: tr.soilLoamDesc,
      imageUrl: '/images/soils/soil-loam.webp?v=4',
      texture: tr.siteConditionsLoamTexture,
      drainage: tr.siteConditionsLoamDrainage,
      ph: tr.siteConditionsLoamPh,
    },
    {
      id: 'CLAY',
      label: tr.soilClay,
      description: tr.soilClayDesc,
      imageUrl: '/images/soils/soil-clay.webp',
      texture: tr.siteConditionsClayTexture,
      drainage: tr.siteConditionsClayDrainage,
      ph: tr.siteConditionsClayPh,
    },
    {
      id: 'SANDY',
      label: tr.soilSandy,
      description: tr.soilSandyDesc,
      imageUrl: '/images/soils/soil-sandy.webp',
      texture: tr.siteConditionsSandyTexture,
      drainage: tr.siteConditionsSandyDrainage,
      ph: tr.siteConditionsSandyPh,
    },
    {
      id: 'CHALKY',
      label: tr.soilChalky,
      description: tr.soilChalkyDesc,
      imageUrl: '/images/soils/soil-chalky.webp',
      texture: tr.siteConditionsChalkyTexture,
      drainage: tr.siteConditionsChalkyDrainage,
      ph: tr.siteConditionsChalkyPh,
    },
    {
      id: 'ACIDIC',
      label: tr.soilAcidic,
      description: tr.soilAcidicDesc,
      imageUrl: '/images/soils/soil-acidic.webp',
      texture: tr.siteConditionsAcidicTexture,
      drainage: tr.siteConditionsAcidicDrainage,
      ph: tr.siteConditionsAcidicPh,
    },
    {
      id: 'SILT',
      label: tr.soilSilt,
      description: tr.soilSiltDesc,
      imageUrl: '/images/soils/soil-silt.webp',
      texture: tr.siteConditionsSiltTexture,
      drainage: tr.siteConditionsSiltDrainage,
      ph: tr.siteConditionsSiltPh,
    },
  ];

  const activeOption = SOIL_OPTIONS.find((s) => s.id === selectedSoil) || SOIL_OPTIONS[0];

  const isOptimal = selectedTree.preferredSoils.includes(selectedSoil);
  const isUnsuitable = selectedTree.unsuitableSoils.includes(selectedSoil);

  const isMoundAdvised =
    selectedSoil === 'CLAY' &&
    (selectedTree.id === 'tree-peach' || selectedTree.id === 'tree-apricot' || selectedTree.id === 'tree-fig');
  const isCalcifugeWarning =
    selectedSoil === 'CHALKY' && selectedTree.id === 'tree-chestnut';

  return (
    <div className="bg-white rounded-2xl p-5 sm:p-6 border border-stone-200/80">
      <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-3 mb-4 pb-4 border-b border-stone-100">
        <div>
          <h2 className="text-lg font-semibold text-stone-900 flex items-center gap-2">
            <Mountain className="w-5 h-5 text-forest-600 shrink-0" strokeWidth={1.75} />
            <span>{tr.soilBarTitle}</span>
          </h2>
          <p className="text-sm text-stone-500 mt-0.5">{tr.soilBarSubtitle}</p>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <button
            type="button"
            onClick={() => setIsMapModalOpen(true)}
            className="flex items-center gap-2 text-xs font-medium h-8 px-3 rounded-lg bg-white hover:bg-stone-50 text-stone-800 border border-stone-200 transition-colors cursor-pointer"
            title={tr.siteConditionsOpenWorldMap}
          >
            <Globe className="w-4 h-4 text-stone-500 shrink-0" strokeWidth={1.75} />
            <span className="font-semibold">{translateClimateZone(selectedZone, language)}</span>
            <span className="text-stone-300 font-normal">•</span>
            <span className="flex items-center gap-1 text-[11px] text-stone-600">
              <Compass className="w-3 h-3 text-stone-500" />
              <span>{hemisphere === 'NORTHERN' ? tr.siteConditionsNorth : tr.siteConditionsSouth}</span>
            </span>
            <span className="text-[11px] text-stone-500 font-normal underline decoration-stone-300 underline-offset-2">
              {tr.siteConditionsWorldMap}
            </span>
          </button>

          {treeAge && onSelectTreeAge && (
            <TreeAgeToggle language={language} treeAge={treeAge} onSelectTreeAge={onSelectTreeAge} />
          )}

        </div>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-2 mb-4">
        {SOIL_OPTIONS.map((soil) => {
          const isSelected = soil.id === selectedSoil;
          const treePrefers = selectedTree.preferredSoils.includes(soil.id);
          const treeStruggles = selectedTree.unsuitableSoils.includes(soil.id);

          return (
            <button
              key={soil.id}
              type="button"
              onClick={() => onSelectSoil(soil.id)}
              className={`p-2.5 rounded-xl text-left flex flex-col justify-between transition relative cursor-pointer overflow-hidden group select-none min-h-[78px] ${
                isSelected
                  ? 'border border-forest-600 ring-1 ring-forest-600'
                  : 'border border-stone-200 hover:border-stone-400 bg-white'
              }`}
            >
              <div className="absolute inset-0 z-0 bg-stone-100">
                <img
                  src={soil.imageUrl}
                  alt={soil.label}
                  className={`w-full h-full object-cover object-center transition duration-300 ${
                    isSelected ? 'opacity-85' : 'opacity-70 group-hover:opacity-85'
                  }`}
                  loading="lazy"
                />
                <div
                  className={`absolute inset-0 transition-colors pointer-events-none ${
                    isSelected
                      ? 'bg-gradient-to-t from-white via-white/80 via-45% to-transparent'
                      : 'bg-gradient-to-t from-white via-white/75 via-45% to-white/10 group-hover:from-white/90 group-hover:via-white/70'
                  }`}
                />
              </div>

              <div className="relative z-10 flex items-center justify-between gap-1 mb-2 w-full">
                {treePrefers ? (
                  <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-white/90 text-[11px] text-emerald-800 font-medium">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-600" />
                    <span>{tr.soilOptimal}</span>
                  </span>
                ) : treeStruggles ? (
                  <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-white/90 text-[11px] text-rose-800 font-medium">
                    <span className="w-1.5 h-1.5 rounded-full bg-rose-600" />
                    <span>{tr.soilUnsuitable}</span>
                  </span>
                ) : (
                  <span />
                )}
                {isSelected && (
                  <span className="w-4 h-4 rounded-full bg-forest-600 text-white flex items-center justify-center ml-auto">
                    <Check className="w-2.5 h-2.5 stroke-[3]" />
                  </span>
                )}
              </div>

              <div className="relative z-10 mt-auto">
                <div
                  className={`text-xs sm:text-[13px] tracking-tight line-clamp-1 ${
                    'font-semibold text-stone-900'
                  }`}
                >
                  {soil.label}
                </div>
                <div
                  className={`text-[11px] line-clamp-1 mt-0.5 ${
                    isSelected ? 'text-forest-800 font-medium' : 'text-stone-600 font-medium'
                  }`}
                >
                  {soil.ph}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <div className="relative rounded-xl bg-stone-50 text-stone-900 p-4 sm:p-5 text-xs">

        <div className="relative z-10 grid grid-cols-1 md:grid-cols-12 gap-3.5 items-center">
          <div className="md:col-span-5 space-y-2">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-lg overflow-hidden shrink-0 bg-stone-100 relative">
                <img
                  src={activeOption.imageUrl}
                  alt={activeOption.label}
                  className="w-full h-full object-cover object-center"
                />
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[11px] text-stone-500">{tr.soilType}</span>
                  <span className="text-stone-300">·</span>
                  <span className="text-[11px] font-medium text-forest-800">
                    {activeOption.ph}
                  </span>
                </div>
                <div className="font-semibold text-stone-900 text-sm mt-0.5 truncate">
                  {activeOption.label}
                </div>
              </div>
            </div>

            <div className="text-stone-700 leading-snug">
              {activeOption.description}
            </div>

            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-stone-600 pt-1.5 border-t border-stone-200">
              <span className="flex items-center gap-1 text-stone-700 font-medium">
                <Droplets className="w-3 h-3 text-stone-500 shrink-0" />
                <span>{activeOption.drainage}</span>
              </span>
              <span className="text-stone-300">•</span>
              <span className="text-stone-700 font-medium">{activeOption.texture}</span>
              <span className="text-stone-300">•</span>
              <span className="flex items-center gap-1 text-stone-700 font-medium">
                <Compass className="w-3 h-3 text-stone-500 shrink-0" />
                <span>{hemisphere === 'NORTHERN' ? tr.siteConditionsNorthSouthSun : tr.siteConditionsSouthNorthSun}</span>
              </span>
            </div>
          </div>

          <div className="md:col-span-7 border-t md:border-t-0 md:border-l border-stone-200 pt-2.5 md:pt-0 md:pl-3.5">
            {isCalcifugeWarning ? (
              <div className="flex items-start gap-2.5 text-rose-950 bg-rose-50 border border-rose-300 p-2.5 rounded-xl">
                <AlertTriangle className="w-4 h-4 text-rose-600 flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-bold text-[11px] text-rose-900">{tr.soilUnsuitable}: {getLoc(selectedTree.commonName, language)}</div>
                  <div className="text-[11px] text-rose-900/90 mt-0.5 leading-relaxed">{tr.soilCalcifugeAlert}</div>
                </div>
              </div>
            ) : isMoundAdvised ? (
              <div className="flex items-start gap-2.5 text-amber-950 bg-amber-50 border border-amber-300 p-2.5 rounded-xl">
                <AlertTriangle className="w-4 h-4 text-amber-600 flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-bold text-[11px] text-amber-900">{tr.soilUnsuitable}: {tr.soilTreeOnSoil.replace('{tree}', getLoc(selectedTree.commonName, language)).replace('{soil}', activeOption.label)}</div>
                  <div className="text-[11px] text-amber-900/90 mt-0.5 leading-relaxed">{tr.soilMoundAdvisory}</div>
                </div>
              </div>
            ) : isOptimal ? (
              <div className="flex items-start gap-2.5 text-emerald-950 bg-emerald-50 border border-emerald-300 p-2.5 rounded-xl">
                <CheckCircle2 className="w-4 h-4 text-emerald-600 flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-bold text-[11px] text-emerald-900">{tr.soilOptimal} ({getLoc(selectedTree.commonName, language)})</div>
                  <div className="text-[11px] text-emerald-900/90 mt-0.5 leading-relaxed">{getLoc(selectedTree.soilAdvice, language)}</div>
                </div>
              </div>
            ) : isUnsuitable ? (
              <div className="flex items-start gap-2.5 text-amber-950 bg-amber-50 border border-amber-300 p-2.5 rounded-xl">
                <Info className="w-4 h-4 text-amber-600 flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-bold text-[11px] text-amber-900">{tr.soilUnsuitable}</div>
                  <div className="text-[11px] text-amber-900/90 mt-0.5 leading-relaxed">{getLoc(selectedTree.soilAdvice, language)}</div>
                </div>
              </div>
            ) : (
              <div className="flex items-start gap-2.5 text-stone-900 bg-white border border-stone-200 p-2.5 rounded-xl">
                <CheckCircle2 className="w-4 h-4 text-sky-600 flex-shrink-0 mt-0.5" />
                <div>
                  <div className="font-bold text-[11px] text-stone-900">{tr.soilTolerant}</div>
                  <div className="text-[11px] text-stone-600 mt-0.5 leading-relaxed">{getLoc(selectedTree.soilAdvice, language)}</div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {isMapModalOpen && (
        <ClimateZoneModal
          language={language}
          selectedZone={selectedZone}
          hemisphere={hemisphere}
          onSelectZone={onSelectZone}
          onSelectHemisphere={onSelectHemisphere}
          onClose={() => setIsMapModalOpen(false)}
        />
      )}
    </div>
  );
};
