import React from 'react';
import { GuildPlant, Language, getLoc } from '../types/guild';
import { calculateGuildEfficiencyScore, calculateRoleCoverage, detectMultiFunctionalHeroes } from '../core/roleCoverageEngine';
import { CheckCircle2, CircleDashed, Sprout, Award } from 'lucide-react';
import { t } from '../i18n/translations';

interface RoleCoverageCardProps {
  language: Language;
  selectedPlants: GuildPlant[];
}

export const RoleCoverageCard: React.FC<RoleCoverageCardProps> = ({ language, selectedPlants }) => {
  const tr = t(language);
  const coverage = calculateRoleCoverage(selectedPlants);
  const efficiency = calculateGuildEfficiencyScore(selectedPlants);
  const multiHeroes = detectMultiFunctionalHeroes(selectedPlants);

  return (
    <div className="bg-white rounded-2xl p-5 sm:p-6 border border-stone-200/80 space-y-4">
      <div className="flex items-center justify-between pb-3 border-b border-stone-100">
        <div>
          <h3 className="text-lg font-semibold text-stone-900 flex items-center gap-2">
            <Award className="w-5 h-5 text-forest-600" strokeWidth={1.75} />
            {tr.coverageTitle}
          </h3>
          <p className="text-sm text-stone-500 mt-0.5">
            {tr.coverageSubtitle}
          </p>
        </div>
        <div className="text-right">
          <div className="text-2xl font-semibold text-stone-900 tabular-nums">
            {efficiency.coveredRoleCount} / {efficiency.totalRoles}
          </div>
          <div className="text-xs text-stone-500 tabular-nums">
            {efficiency.efficiencyPercent}% {tr.coveredRatio}
          </div>
        </div>
      </div>

      <div className="w-full bg-stone-100 rounded-full h-2 overflow-hidden">
        <div
          className={`h-full transition duration-500 rounded-full ${
            efficiency.isFullyCovered ? 'bg-forest-600' : 'bg-forest-500'
          }`}
          style={{ width: `${efficiency.efficiencyPercent}%` }}
        />
      </div>

      {multiHeroes.length > 0 && (
        <div className="p-4 rounded-xl bg-stone-50">
          <div className="flex items-center gap-1.5 text-xs font-semibold text-stone-800 mb-2">
            <Sprout className="w-3.5 h-3.5 text-forest-600" strokeWidth={1.75} />
            <span>{tr.heroesTitle}</span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {multiHeroes.map(({ plant, roleCount }) => (
              <span
                key={plant.id}
                className="px-2 py-0.5 rounded-md bg-white border border-stone-200 text-stone-800 text-xs flex items-center gap-1.5"
                title={`${getLoc(plant.commonName, language)}: ${roleCount} ${tr.rolesStacked}`}
              >
                <span>{getLoc(plant.commonName, language)}</span>
                <span className="text-[11px] text-stone-500 tabular-nums">
                  {roleCount}
                </span>
              </span>
            ))}
          </div>
          <p className="text-xs text-stone-500 mt-2">
            {tr.heroesTip}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2 pt-1">
        {coverage.map((item) => {
          return (
            <div
              key={item.role}
              className={`p-3 rounded-xl border text-xs flex items-start justify-between gap-2 ${
                item.covered
                  ? 'border-stone-200 bg-white text-stone-900'
                  : 'border-dashed border-stone-200 bg-stone-50 text-stone-500'
              }`}
            >
              <div className="flex items-start gap-2">
                {item.covered ? (
                  <CheckCircle2 className="w-4 h-4 text-forest-600 mt-0.5 flex-shrink-0" />
                ) : (
                  <CircleDashed className="w-4 h-4 text-stone-300 mt-0.5 flex-shrink-0" />
                )}
                <div>
                  <div className="font-semibold">{getLoc(item.displayName, language)}</div>
                  <div className="text-[11px] text-stone-500 line-clamp-1">
                    {item.covered
                      ? item.plants.map(p => getLoc(p.commonName, language)).join(', ')
                      : tr.notYetCovered}
                  </div>
                </div>
              </div>

              {item.covered && (
                <span className="text-[11px] text-stone-500 tabular-nums shrink-0">
                  {item.plantCount}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
