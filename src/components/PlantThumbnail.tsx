import React, { useState } from 'react';

interface PlantThumbnailProps {
  src?: string;
  alt: string;
  fallbackText?: string;
  fallbackColor?: string;
  className?: string;
  roundedClassName?: string;
  targetSize?: number;
  priority?: 'high' | 'low' | 'auto';
}

// Only Unsplash URLs get resized; local assets are pre-sized and Wikimedia rejects unknown params.
function getOptimizedImageUrl(url: string, targetSize: number): string {
  if (url.includes('images.unsplash.com')) {
    if (url.includes('w=')) {
      return url.replace(/w=\d+/, `w=${targetSize}&q=75&auto=format`);
    } else {
      const sep = url.includes('?') ? '&' : '?';
      return `${url}${sep}w=${targetSize}&q=75&auto=format`;
    }
  }

  return url;
}

/** Dark ink on light plant colours (yellow, lime), white on dark ones, so initials stay legible. */
function initialsColor(bg: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(bg.trim());
  if (!m) return '#ffffff';
  const [r, g, b] = [0, 2, 4].map(i => parseInt(m[1].slice(i, i + 2), 16) / 255)
    .map(c => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  // Pick whichever of ink (#2e2922, luminance ~0.024) and white gives more contrast
  const onWhite = 1.05 / (lum + 0.05);
  const onInk = (lum + 0.05) / 0.074;
  return onInk > onWhite ? '#2e2922' : '#ffffff';
}

export const PlantThumbnail: React.FC<PlantThumbnailProps> = ({
  src,
  alt,
  fallbackText,
  fallbackColor = '#15803d',
  className = 'w-10 h-10',
  roundedClassName = 'rounded-xl',
  targetSize = 160,
  priority = 'auto',
}) => {
  // Keyed by src so a reused instance (e.g. the selected-tree preview) resets when the image changes.
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const [loadedSrc, setLoadedSrc] = useState<string | null>(null);
  const hasError = !!src && failedSrc === src;
  const isLoaded = !!src && loadedSrc === src;

  const initials = fallbackText
    ? fallbackText.substring(0, 2).toUpperCase()
    : alt.substring(0, 2).toUpperCase();

  if (!src || hasError) {
    return (
      <div
        className={`${className} ${roundedClassName} flex items-center justify-center text-xs font-bold flex-shrink-0 shadow-xs select-none`}
        style={{ backgroundColor: fallbackColor, color: initialsColor(fallbackColor) }}
        title={alt}
      >
        {initials}
      </div>
    );
  }

  const optimizedSrc = getOptimizedImageUrl(src, targetSize);
  const optimized2x = getOptimizedImageUrl(src, targetSize * 2);
  const isHighPriority = priority === 'high';

  return (
    <div className={`relative ${className} ${roundedClassName} overflow-hidden flex-shrink-0 bg-stone-100 shadow-xs`}>
      {!isLoaded && (
        <div
          className="absolute inset-0 flex items-center justify-center text-[11px] font-bold"
          style={{ backgroundColor: fallbackColor, color: initialsColor(fallbackColor) }}
        >
          {initials}
        </div>
      )}
      <img
        src={optimizedSrc}
        srcSet={optimized2x !== optimizedSrc ? `${optimizedSrc} 1x, ${optimized2x} 2x` : undefined}
        alt={alt}
        loading={isHighPriority ? 'eager' : 'lazy'}
        {...{ fetchpriority: isHighPriority ? 'high' : priority === 'low' ? 'low' : undefined }}
        decoding={isHighPriority ? 'sync' : 'async'}
        onLoad={() => setLoadedSrc(src)}
        onError={() => setFailedSrc(src)}
        className={`w-full h-full object-cover transition-opacity duration-200 ${
          isLoaded ? 'opacity-100' : 'opacity-0'
        }`}
      />
    </div>
  );
};
