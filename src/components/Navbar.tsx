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
            <div className="w-9 h-9 rounded-xl bg-forest-700 text-cream flex items-center justify-center shrink-0">
              <Sprout className="w-5 h-5" strokeWidth={1.75} />
            </div>
            <div className="hidden sm:block shrink-0">
              <div className="text-lg sm:text-xl font-semibold text-stone-900 tracking-[-0.02em] flex items-center gap-2 group-hover:text-forest-800 transition-colors whitespace-nowrap">
                <span>pflanzengilde<span className="text-forest-600">.de</span></span>
              </div>
            </div>
          </button>

          <nav className="flex items-center bg-white/70 p-1 rounded-xl text-sm shrink-0 ring-1 ring-stone-900/5">
            <button
              type="button"
              onClick={() => onNavigate('/')}
              aria-label={tr.navPlanner}
              className={`h-8 px-2 sm:px-3 rounded-lg font-medium transition-colors duration-150 cursor-pointer flex items-center gap-2 whitespace-nowrap ${
                isPlanner
                  ? 'bg-stone-900 text-cream'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-900/5'
              }`}
            >
              <Sprout className="w-4 h-4 shrink-0" strokeWidth={1.75} />
              <span className="hidden sm:inline">{tr.navPlanner}</span>
            </button>
            <button
              type="button"
              onClick={() => onNavigate('/garten')}
              aria-label={tr.navGarden}
              className={`h-8 px-2 sm:px-3 rounded-lg font-medium transition-colors duration-150 cursor-pointer flex items-center gap-2 whitespace-nowrap ${
                isGarden
                  ? 'bg-stone-900 text-cream'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-900/5'
              }`}
            >
              <Grid className="w-4 h-4 shrink-0" strokeWidth={1.75} />
              <span className="hidden sm:inline">{tr.navGarden}</span>
            </button>
            <button
              type="button"
              onClick={() => onNavigate('/guides')}
              aria-label={tr.navGuides}
              className={`h-8 px-2 sm:px-3 rounded-lg font-medium transition-colors duration-150 cursor-pointer flex items-center gap-2 whitespace-nowrap ${
                currentPath.startsWith('/guides')
                  ? 'bg-stone-900 text-cream'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-900/5'
              }`}
            >
              <BookOpen className="w-4 h-4 shrink-0" strokeWidth={1.75} />
              <span className="hidden sm:inline">{tr.navGuides}</span>
            </button>
          </nav>
        </div>

        <div className="flex items-center gap-1 sm:gap-2 shrink-0">
          {showActions && (
            <>
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                className="hidden xl:flex h-9 px-3 rounded-lg text-sm font-medium text-stone-600 hover:bg-stone-900/5 hover:text-stone-900 items-center gap-2 transition-colors cursor-pointer"
                title={tr.importPlan}
              >
                <Upload className="w-4 h-4 shrink-0" strokeWidth={1.75} />
                <span>{tr.importPlan}</span>
              </button>

              {onShare && (
                <button
                  type="button"
                  onClick={onShare}
                  className="hidden lg:flex h-9 px-3 rounded-lg text-sm font-medium text-stone-600 hover:bg-stone-900/5 hover:text-stone-900 items-center gap-2 transition-colors cursor-pointer shrink-0"
                  title={isGarden ? tr.navbarShareGarden : tr.shareGuildTitle}
                >
                  <Share2 className="w-4 h-4 shrink-0" strokeWidth={1.75} />
                  <span>{isGarden ? tr.navbarShareGarden : tr.shareGuildBtn}</span>
                </button>
              )}

              {/* Opens on hover (desktop) or click (touch) */}
              <div ref={exportRef} className="relative group shrink-0">
                <button
                  type="button"
                  onClick={() => setOpenMenu(prev => prev === 'export' ? null : 'export')}
                  aria-expanded={openMenu === 'export'}
                  className="h-9 px-3 sm:px-4 rounded-lg bg-forest-700 text-cream text-sm font-medium hover:bg-forest-800 flex items-center gap-2 transition-colors cursor-pointer"
                >
                  <Download className="w-4 h-4 shrink-0" strokeWidth={1.75} />
                  <span className="hidden sm:inline">{tr.exportMenu}</span>
                </button>
                <div
                  className={`absolute right-0 top-full pt-1.5 ${isGarden ? 'w-64' : 'w-56'} z-50 animate-in fade-in zoom-in-95 ${
                    openMenu === 'export' ? 'block' : 'hidden group-hover:block'
                  }`}
                >
                  <div className="bg-white rounded-xl ring-1 ring-stone-900/5 shadow-[0_12px_32px_-12px_rgba(46,41,34,0.2)] p-1.5 relative before:absolute before:-top-3 before:left-0 before:right-0 before:h-3 before:content-['']">
                    <div className="px-3 py-1 text-xs font-medium text-stone-500">
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
                      <FileText className="w-4 h-4 text-forest-700 shrink-0" />
                      <div>
                        <div className="font-medium text-stone-800">{tr.exportPdf}</div>
                        <div className="text-xs text-stone-500">
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
                      <Calendar className="w-4 h-4 text-forest-700 shrink-0" />
                      <div>
                        <div className="font-medium text-stone-800">{tr.exportCalendar}</div>
                        <div className="text-xs text-stone-500">
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
                      <FileJson className="w-4 h-4 text-forest-700 shrink-0" />
                      <div>
                        <div className="font-medium text-stone-800">{tr.exportJson}</div>
                        <div className="text-xs text-stone-500">
                          {isGarden
                            ? tr.navbarJsonGardenSub
                            : tr.navbarJsonGuildSub}
                        </div>
                      </div>
                    </button>
                    {isGarden && (
                      <>
                        <div className="mt-1 pt-1.5 border-t border-stone-100 px-3 py-1 text-xs font-medium text-stone-500">
                          {tr.navbarExportCadSection}
                        </div>
                        {([
                          { event: 'garden-export-dxf', Icon: DraftingCompass, color: 'text-forest-700', title: tr.navbarExportDxf, sub: tr.navbarExportDxfSub },
                          { event: 'garden-export-svg', Icon: PenTool, color: 'text-forest-700', title: tr.navbarExportSvg, sub: tr.navbarExportSvgSub },
                          { event: 'garden-export-csv', Icon: Sheet, color: 'text-forest-700', title: tr.navbarExportCsv, sub: tr.navbarExportCsvSub },
                          { event: 'garden-export-geojson', Icon: Globe, color: 'text-forest-700', title: tr.navbarExportGeoJson, sub: tr.navbarExportGeoJsonSub },
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
                              <div className="font-medium text-stone-800">{title}</div>
                              <div className="text-xs text-stone-500">{sub}</div>
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
                className="hidden xl:flex w-9 h-9 items-center justify-center rounded-lg text-stone-500 hover:text-stone-900 hover:bg-stone-900/5 transition-colors cursor-pointer shrink-0"
                title={tr.resetGuild}
              >
                <RotateCcw className="w-4 h-4" />
              </button>

              <div ref={mobileMenuRef} className="relative xl:hidden shrink-0">
                <button
                  type="button"
                  onClick={() => setOpenMenu(prev => prev === 'mobile-more' ? null : 'mobile-more')}
                  aria-expanded={openMenu === 'mobile-more'}
                  className="w-9 h-9 flex items-center justify-center rounded-lg text-stone-600 hover:bg-stone-900/5 transition-colors cursor-pointer"
                  title={tr.moreOptions}
                  aria-label={tr.moreOptions}
                >
                  <MoreVertical className="w-4 h-4" />
                </button>

                {openMenu === 'mobile-more' && (
                  <div className="absolute right-0 top-full pt-1.5 w-52 z-50 animate-in fade-in zoom-in-95">
                    <div className="bg-white rounded-xl ring-1 ring-stone-900/5 shadow-[0_12px_32px_-12px_rgba(46,41,34,0.2)] p-2 space-y-1">
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

          <div className={`flex items-center gap-0.5 text-xs shrink-0 ${showActions ? "ml-1 pl-2 sm:pl-3 border-l border-stone-900/10" : ""}`}>
            <button
              type="button"
              onClick={() => setLanguage('de')}
              className={`h-7 px-2 rounded-md font-medium transition-colors cursor-pointer ${
                language === 'de'
                  ? 'bg-stone-900/5 text-stone-900'
                  : 'text-stone-500 hover:text-stone-900'
              }`}
            >
              DE
            </button>
            <button
              type="button"
              onClick={() => setLanguage('en')}
              className={`h-7 px-2 rounded-md font-medium transition-colors cursor-pointer ${
                language === 'en'
                  ? 'bg-stone-900/5 text-stone-900'
                  : 'text-stone-500 hover:text-stone-900'
              }`}
            >
              EN
            </button>
            <button
              type="button"
              onClick={() => setLanguage('nb')}
              className={`h-7 px-2 rounded-md font-medium transition-colors cursor-pointer ${
                language === 'nb'
                  ? 'bg-stone-900/5 text-stone-900'
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
