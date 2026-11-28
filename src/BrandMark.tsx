import { useId } from 'react';

/** The "nl" monogram on its dark plate, as in the app icon (src-tauri/icon.svg). Size and colours come from tokens.css. */
export function BrandMark() {
  // A gradient is found by its id across the whole page: each mark needs its own.
  // React's ids hold characters such as ':' that a url(#…) reference may not parse; keep letters and digits.
  const gradient = `brand-gradient-${useId().replace(/[^a-zA-Z0-9]/g, '')}`;
  return (
    <svg className="brand-mark" viewBox="0 0 1024 1024" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id={gradient} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" className="brand-gradient-start" />
          <stop offset="1" className="brand-gradient-end" />
        </linearGradient>
      </defs>
      <rect className="brand-plate" width="1024" height="1024" rx="224" />
      <path
        d="M280 784 V512 A132 132 0 0 1 544 512 V784 M744 240 V784"
        fill="none"
        stroke={`url(#${gradient})`}
        strokeWidth="120"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
