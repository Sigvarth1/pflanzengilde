import React from 'react';
import { ArrowDown, ArrowUpRight } from 'lucide-react';

interface HeroBannerProps {
  title: string;
  subtitle: string;
  primaryLabel: string;
  onPrimary: () => void;
  secondaryLabel?: string;
  onSecondary?: () => void;
  /** Smaller variant for sub-pages such as the guides. */
  compact?: boolean;
  /** 'down' scrolls within the page, 'out' goes to another page. */
  primaryDirection?: 'down' | 'out';
}

/**
 * Dark rounded hero card with slow-drifting ribbon bands (Phantom-inspired).
 * The bands are authored SVG paths, read as roots and growth lines.
 */
export const HeroBanner: React.FC<HeroBannerProps> = ({
  title,
  subtitle,
  primaryLabel,
  onPrimary,
  secondaryLabel,
  onSecondary,
  compact = false,
  primaryDirection = 'down',
}) => {
  return (
    <section
      className={`relative overflow-hidden rounded-[28px] sm:rounded-[36px] bg-stone-950 text-cream isolate ${
        compact ? 'px-6 py-12 sm:px-12 sm:py-16' : 'px-6 py-16 sm:px-12 sm:py-24'
      }`}
    >
      <svg
        className="hero-ribbons absolute inset-0 -z-10 h-full w-full"
        viewBox="0 0 1200 520"
        preserveAspectRatio="xMidYMid slice"
        aria-hidden="true"
      >
        <g fill="none" strokeLinecap="round">
          <path className="hero-ribbon hero-ribbon-a" d="M-120 470 C 160 360, 300 120, 560 140 S 980 380, 1320 60" stroke="#2f6b4f" strokeWidth="64" />
          <path className="hero-ribbon hero-ribbon-b" d="M-140 120 C 140 220, 360 40, 620 200 S 1000 480, 1340 330" stroke="#ab9ff2" strokeWidth="58" opacity="0.9" />
          <path className="hero-ribbon hero-ribbon-c" d="M-100 330 C 200 520, 420 300, 700 420 S 1060 160, 1320 240" stroke="#c9a227" strokeWidth="40" opacity="0.85" />
          <path className="hero-ribbon hero-ribbon-d" d="M60 -60 C 220 160, 520 260, 760 60 S 1100 -40, 1260 140" stroke="#2c4fa3" strokeWidth="50" opacity="0.9" />
          <path className="hero-ribbon hero-ribbon-e" d="M-80 40 C 260 60, 420 400, 820 300 S 1180 520, 1300 520" stroke="#8f8aa3" strokeWidth="22" opacity="0.55" />
        </g>
      </svg>
      <div className="pointer-events-none absolute inset-0 -z-10 bg-[radial-gradient(ellipse_at_center,rgba(34,26,56,0.78)_0%,rgba(34,26,56,0.35)_55%,rgba(34,26,56,0)_80%)]" />

      <div className="relative mx-auto max-w-3xl text-center">
        <h1
          className={`font-normal text-cream tracking-[-0.035em] leading-[1.02] ${
            compact ? 'text-4xl sm:text-6xl' : 'text-[44px] sm:text-7xl lg:text-[84px]'
          }`}
        >
          {title}
        </h1>
        <p className="mx-auto mt-5 max-w-xl text-base sm:text-lg text-stone-200 leading-relaxed">
          {subtitle}
        </p>
        <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
          <button
            type="button"
            onClick={onPrimary}
            className="inline-flex items-center gap-2 rounded-full bg-forest-400 px-6 py-3 text-[15px] font-semibold text-stone-900 transition-colors duration-200 hover:bg-forest-300 cursor-pointer"
          >
            <span>{primaryLabel}</span>
            {primaryDirection === 'down' ? <ArrowDown className="h-4 w-4" /> : <ArrowUpRight className="h-4 w-4" />}
          </button>
          {secondaryLabel && onSecondary && (
            <button
              type="button"
              onClick={onSecondary}
              className="inline-flex items-center gap-2 rounded-full bg-cream/10 px-6 py-3 text-[15px] font-semibold text-cream backdrop-blur transition-colors duration-200 hover:bg-cream/20 cursor-pointer"
            >
              <span>{secondaryLabel}</span>
              <ArrowUpRight className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>
    </section>
  );
};
