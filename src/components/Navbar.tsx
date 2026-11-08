import React, { useState, useRef, useEffect } from 'react';
import { Language } from '../types/guild';
import { Sprout, RotateCcw, Download, Upload, FileText, FileJson, Calendar, BookOpen, MoreVertical, Share2, Grid, DraftingCompass, PenTool, Sheet, Globe } from 'lucide-react';
import { t } from '../i18n/translations';

interface NavbarProps {
  language: Language;
  setLanguage: (l: Language) => void;
  currentPath: string;
  onNavigate: (path: string) => void;
  onReset: () => void;
  onLoadPreset?: (presetName: 'apple' | 'walnut' | 'apricot' | 'minimal') => void;
  onExportJson: () => void;
  onExportPdf: () => void;
  onExportCalendar: () => void;
  onImportJson: (file: File) => void;
  onShare?: () => void;
}

export const Navbar: React.FC<NavbarProps> = ({
  language,
  setLanguage,
  currentPath,
  onNavigate,
  onReset,
  onLoadPreset,
  onExportJson,
  onExportPdf,
  onExportCalendar,
  onImportJson,
  onShare,
}) => {
  const tr = t(language);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const exportRef = useRef<HTMLDivElement>(null);
  const mobileMenuRef = useRef<HTMLDivElement>(null);

  // currentPath carries query and hash (e.g. "/?g=..." from a share link)
  const pathname = currentPath.split(/[?#]/)[0];
  const isPlanner = pathname === '/' || pathname === '';
  const isGarden = currentPath.startsWith('/garten') || currentPath.startsWith('/garden');
  const showActions = isPlanner || isGarden;

  const [openMenu, setOpenMenu] = useState<'export' | 'mobile-more' | null>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (
        exportRef.current && !exportRef.current.contains(target) &&
        mobileMenuRef.current && !mobileMenuRef.current.contains(target)
      ) {
        setOpenMenu(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      for (let i = 0; i < files.length; i++) {
        onImportJson(files[i]);
      }
      e.target.value = '';
    }
  };

  return (
    <header className="sticky top-0 z-40 bg-stone-100/85 backdrop-blur-md">
      <div className="max-w-7xl mx-auto px-3 sm:px-6 lg:px-8 h-[72px] flex items-center justify-between gap-2 sm:gap-4">
        <div className="flex items-center gap-2 sm:gap-6 shrink-0 min-w-0">
          <button
            type="button"
            onClick={() => onNavigate('/')}
            className="flex items-center space-x-2 sm:space-x-3 text-left focus:outline-none cursor-pointer group shrink-0"
            title={tr.appTitle}
          >
            <div className="w-9 h-9 sm:w-10 sm:h-10 rounded-full bg-forest-400 text-stone-900 flex items-center justify-center group-hover:rotate-[-8deg] transition-transform duration-300 ease-out shrink-0">
              <Sprout className="w-5 h-5 sm:w-6 sm:h-6" />
            </div>
            <div className="hidden sm:block shrink-0">
              <div className="text-lg sm:text-[22px] font-bold text-stone-900 tracking-[-0.03em] flex items-center gap-2 group-hover:text-forest-800 transition-colors whitespace-nowrap">
                <span>pflanzengilde<span className="text-forest-500">.de</span></span>
                <span className="text-[11px] font-semibold px-2.5 py-0.5 rounded-full bg-white text-stone-600 tracking-[-0.01em] shrink-0">
                  {tr.badge}
                </span>
              </div>
            </div>
          </button>

          <nav className="flex items-center bg-white p-1 rounded-full text-[13px] shrink-0">
            <button
              type="button"
              onClick={() => onNavigate('/')}
              aria-label={tr.navPlanner}
              className={`px-2 sm:px-4 py-2 rounded-full font-semibold transition-colors duration-200 cursor-pointer flex items-center gap-1.5 whitespace-nowrap ${
                isPlanner
                  ? 'bg-forest-400 text-stone-900'
                  : 'text-stone-500 hover:text-stone-900 hover:bg-stone-100'
              }`}
            >
              <Sprout className="w-3.5 h-3.5 shrink-0" />
              <span className="hidden sm:inline">{tr.navPlanner}</span>
            </button>
            <button
              type="button"
              onClick={() => onNavigate('/garten')}
              aria-label={tr.navGarden}
              className={`px-2 sm:px-4 py-2 rounded-full font-semibold transition-colors duration-200 cursor-pointer flex items-center gap-1.5 whitespace-nowrap ${
                isGarden
                  ? 'bg-forest-400 text-stone-900'
                  : 'text-stone-500 hover:text-stone-900 hover:bg-stone-100'
              }`}
            >
              <Grid className="w-3.5 h-3.5 shrink-0" />
              <span className="hidden sm:inline">{tr.navGarden}</span>
            </button>
            <button
              type="button"
              onClick={() => onNavigate('/guides')}
              aria-label={tr.navGuides}
              className={`px-2 sm:px-4 py-2 rounded-full font-semibold transition-colors duration-200 cursor-pointer flex items-center gap-1.5 whitespace-nowrap ${
                currentPath.startsWith('/guides')
                  ? 'bg-forest-400 text-stone-900'
                  : 'text-stone-500 hover:text-stone-900 hover:bg-stone-100'
              }`}
            >
              <BookOpen className="w-3.5 h-3.5 shrink-0" />
              <span className="hidden sm:inline">{tr.navGuides}</span>
            </button>
          </nav>
        </div>

        <div className="flex items-center space-x-1.5 sm:space-x-2.5 shrink-0">
          {showActions && (
            <>
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                className="hidden xl:flex px-4 py-2 rounded-full bg-white text-[13px] font-semibold text-stone-700 hover:bg-forest-50 hover:text-stone-900 items-center gap-1.5 transition-colors cursor-pointer"
                title={tr.importPlan}
              >
                <Upload className="w-3.5 h-3.5 text-stone-600 shrink-0" />
                <span>{tr.importPlan}</span>
              </button>

              {onShare && (
                <button
                  type="button"
                  onClick={onShare}
                  className="hidden lg:flex px-4 py-2 rounded-full bg-white text-[13px] font-semibold text-stone-700 hover:bg-forest-50 hover:text-stone-900 items-center gap-1.5 transition-colors cursor-pointer shrink-0"
                  title={isGarden ? tr.navbarShareGarden : tr.shareGuildTitle}
                >
                  <Share2 className="w-3.5 h-3.5 text-forest-600 shrink-0" />
                  <span>{isGarden ? tr.navbarShareGarden : tr.shareGuildBtn}</span>
                </button>
              )}

              {/* Opens on hover (desktop) or click (touch) */}
              <div ref={exportRef} className="relative group shrink-0">
                <button
                  type="button"
                  onClick={() => setOpenMenu(prev => prev === 'export' ? null : 'export')}
                  aria-expanded={openMenu === 'export'}
                  className="px-3.5 sm:px-5 py-2 rounded-full bg-forest-400 text-stone-900 text-[13px] font-semibold hover:bg-forest-300 flex items-center gap-1.5 transition-colors cursor-pointer"
                >
                  <Download className="w-3.5 h-3.5 shrink-0" />
                  <span className="hidden sm:inline">{tr.exportMenu}</span>
                </button>
                <div
                  className={`absolute right-0 top-full pt-1.5 ${isGarden ? 'w-64' : 'w-56'} z-50 animate-in fade-in zoom-in-95 ${
                    openMenu === 'export' ? 'block' : 'hidden group-hover:block'
                  }`}
                >
                  <div className="bg-white rounded-2xl shadow-[0_18px_40px_-12px_rgba(60,49,91,0.28)] p-1.5 relative before:absolute before:-top-3 before:left-0 before:right-0 before:h-3 before:content-['']">
                    <div className="px-3 py-1 text-[10px] font-bold text-stone-400 uppercase tracking-wider">
                      {isGarden ? tr.navbarExportGardenPlan : tr.exportPlan}
                    </div>
                    <button
                      type="button"
                      onClick={() => {
                        onExportPdf();
                        setOpenMenu(null);
                      }}
                      className="w-full text-left px-3 py-2 rounded-xl text-xs text-stone-700 hover:bg-stone-100 hover:text-stone-900 flex items-center gap-2.5 transition-colors cursor-pointer"
                    >
                      <FileText className="w-4 h-4 text-rose-600 shrink-0" />
                      <div>
                        <div className="font-semibold text-stone-800">{tr.exportPdf}</div>
                        <div className="text-[10px] text-stone-500">
                          {isGarden
                            ? tr.navbarPdfGardenSub
                            : tr.navbarPdfGuildSub}
                        </div>
                      </div>
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        onExportCalendar();
                        setOpenMenu(null);
                      }}
                      className="w-full text-left px-3 py-2 rounded-xl text-xs text-stone-700 hover:bg-stone-100 hover:text-stone-900 flex items-center gap-2.5 transition-colors cursor-pointer"
                    >
                      <Calendar className="w-4 h-4 text-amber-600 shrink-0" />
                      <div>
                        <div className="font-semibold text-stone-800">{tr.exportCalendar}</div>
                        <div className="text-[10px] text-stone-500">
                          {isGarden
                            ? tr.navbarCalendarGardenSub
                            : tr.exportCalendarSub}
                        </div>
                      </div>
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        onExportJson();
                        setOpenMenu(null);
                      }}
                      className="w-full text-left px-3 py-2 rounded-xl text-xs text-stone-700 hover:bg-stone-100 hover:text-stone-900 flex items-center gap-2.5 transition-colors cursor-pointer"
                    >
                      <FileJson className="w-4 h-4 text-emerald-600 shrink-0" />
                      <div>
                        <div className="font-semibold text-stone-800">{tr.exportJson}</div>
                        <div className="text-[10px] text-stone-500">
                          {isGarden
                            ? tr.navbarJsonGardenSub
                            : tr.navbarJsonGuildSub}
                        </div>
                      </div>
                    </button>
                    {isGarden && (
                      <>
                        <div className="mt-1 pt-1.5 border-t border-stone-100 px-3 py-1 text-[10px] font-bold text-stone-400 uppercase tracking-wider">
                          {tr.navbarExportCadSection}
                        </div>
                        {([
                          { event: 'garden-export-dxf', Icon: DraftingCompass, color: 'text-sky-700', title: tr.navbarExportDxf, sub: tr.navbarExportDxfSub },
                          { event: 'garden-export-svg', Icon: PenTool, color: 'text-violet-600', title: tr.navbarExportSvg, sub: tr.navbarExportSvgSub },
                          { event: 'garden-export-csv', Icon: Sheet, color: 'text-emerald-700', title: tr.navbarExportCsv, sub: tr.navbarExportCsvSub },
                          { event: 'garden-export-geojson', Icon: Globe, color: 'text-teal-600', title: tr.navbarExportGeoJson, sub: tr.navbarExportGeoJsonSub },
                        ] as const).map(({ event, Icon, color, title, sub }) => (
                          <button
                            key={event}
                            type="button"
                            onClick={() => {
                              window.dispatchEvent(new CustomEvent(event));
                              setOpenMenu(null);
                            }}
                            className="w-full text-left px-3 py-2 rounded-xl text-xs text-stone-700 hover:bg-stone-100 hover:text-stone-900 flex items-center gap-2.5 transition-colors cursor-pointer"
                          >
                            <Icon className={`w-4 h-4 ${color} shrink-0`} />
                            <div>
                              <div className="font-semibold text-stone-800">{title}</div>
                              <div className="text-[10px] text-stone-500">{sub}</div>
                            </div>
                          </button>
                        ))}
                      </>
                    )}
                  </div>
                </div>
              </div>

              <button
                type="button"
                onClick={onReset}
                className="hidden xl:flex w-9 h-9 items-center justify-center rounded-full bg-white text-stone-500 hover:text-stone-900 hover:bg-forest-50 transition-colors cursor-pointer shrink-0"
                title={tr.resetGuild}
              >
                <RotateCcw className="w-4 h-4" />
              </button>

              <div ref={mobileMenuRef} className="relative xl:hidden shrink-0">
                <button
                  type="button"
                  onClick={() => setOpenMenu(prev => prev === 'mobile-more' ? null : 'mobile-more')}
                  aria-expanded={openMenu === 'mobile-more'}
                  className="w-9 h-9 flex items-center justify-center rounded-full bg-white text-stone-700 hover:bg-forest-50 transition-colors cursor-pointer"
                  title={tr.moreOptions}
                  aria-label={tr.moreOptions}
                >
                  <MoreVertical className="w-4 h-4" />
                </button>

                {openMenu === 'mobile-more' && (
                  <div className="absolute right-0 top-full pt-1.5 w-52 z-50 animate-in fade-in zoom-in-95">
                    <div className="bg-white rounded-2xl shadow-[0_18px_40px_-12px_rgba(60,49,91,0.28)] p-2 space-y-1">
                      {onShare && (
                        <button
                          type="button"
                          onClick={() => {
                            onShare();
                            setOpenMenu(null);
                          }}
                          className="w-full flex items-center gap-2 p-2 rounded-lg hover:bg-stone-50 text-xs font-medium text-stone-700"
                        >
                          <Share2 className="w-4 h-4 text-forest-600" />
                          <span>{isGarden ? tr.navbarShareGarden : tr.shareGuildTitle}</span>
                        </button>
                      )}

                      <button
                        type="button"
                        onClick={() => {
                          fileInputRef.current?.click();
                          setOpenMenu(null);
                        }}
                        className="w-full flex items-center gap-2 p-2 rounded-lg hover:bg-stone-50 text-xs font-medium text-stone-700"
                      >
                        <Upload className="w-4 h-4 text-stone-600" />
                        <span>{tr.importPlan}</span>
                      </button>

                      <button
                        type="button"
                        onClick={() => {
                          onReset();
                          setOpenMenu(null);
                        }}
                        className="w-full flex items-center gap-2 p-2 rounded-lg hover:bg-rose-50 text-xs font-medium text-rose-700"
                      >
                        <RotateCcw className="w-4 h-4 text-rose-600" />
                        <span>{tr.resetGuild}</span>
                      </button>
                    </div>
                  </div>
                )}
              </div>

              
            </>
          )}

          <div className="flex items-center bg-white rounded-full p-0.5 sm:p-1 text-xs shrink-0">
            <button
              type="button"
              onClick={() => setLanguage('de')}
              className={`px-2 sm:px-2.5 py-1.5 rounded-full font-semibold transition-colors cursor-pointer ${
                language === 'de'
                  ? 'bg-stone-900 text-cream font-bold'
                  : 'text-stone-500 hover:text-stone-900'
              }`}
            >
              DE
            </button>
            <button
              type="button"
              onClick={() => setLanguage('en')}
              className={`px-2 sm:px-2.5 py-1.5 rounded-full font-semibold transition-colors cursor-pointer ${
                language === 'en'
                  ? 'bg-stone-900 text-cream font-bold'
                  : 'text-stone-500 hover:text-stone-900'
              }`}
            >
              EN
            </button>
            <button
              type="button"
              onClick={() => setLanguage('nb')}
              className={`px-2 sm:px-2.5 py-1.5 rounded-full font-semibold transition-colors cursor-pointer ${
                language === 'nb'
                  ? 'bg-stone-900 text-cream font-bold'
                  : 'text-stone-500 hover:text-stone-900'
              }`}
            >
              NO
            </button>
          </div>
        </div>
      </div>

      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileChange}
        accept=".json,application/json"
        multiple
        className="hidden"
      />
    </header>
  );
};
