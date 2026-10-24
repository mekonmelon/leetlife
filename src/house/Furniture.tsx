// Each piece of furniture is a small SVG drawn at a fixed spot in the room.
// `asset_path` from the database picks the renderer, so buying an item in Rust
// is all it takes for it to appear here.
import type { JSX } from "react";

type Piece = { x: number; y: number; draw: () => JSX.Element };

const plant = () => (
  <g>
    <path d="M0 26 L4 12 L16 12 L20 26 Z" fill="#b5651d" />
    <path d="M10 12 C10 2 2 2 3 -4 C9 -3 11 3 10 12 Z" fill="#4f9d5a" />
    <path d="M10 12 C10 1 18 1 17 -5 C11 -4 9 3 10 12 Z" fill="#3f8a4b" />
  </g>
);

const desk = () => (
  <g>
    <rect x="0" y="6" width="54" height="5" rx="2" fill="#8a5a3b" />
    <rect x="3" y="11" width="4" height="20" fill="#6f4529" />
    <rect x="47" y="11" width="4" height="20" fill="#6f4529" />
    <rect x="16" y="-6" width="22" height="13" rx="2" fill="#2b3350" stroke="#59627f" />
    <rect x="18" y="-4" width="18" height="9" fill="#7ea6ff" opacity="0.65" />
  </g>
);

const bed = () => (
  <g>
    <rect x="0" y="8" width="56" height="16" rx="3" fill="#7d6bb0" />
    <rect x="0" y="2" width="16" height="14" rx="3" fill="#efe6da" />
    <rect x="16" y="6" width="40" height="10" rx="3" fill="#9b8ac9" />
    <rect x="2" y="24" width="4" height="6" fill="#5b4d80" />
    <rect x="50" y="24" width="4" height="6" fill="#5b4d80" />
  </g>
);

const couch = () => (
  <g>
    <rect x="0" y="6" width="48" height="16" rx="5" fill="#c4674f" />
    <rect x="-4" y="2" width="10" height="20" rx="4" fill="#a9553f" />
    <rect x="42" y="2" width="10" height="20" rx="4" fill="#a9553f" />
    <rect x="6" y="4" width="16" height="8" rx="3" fill="#dc836c" />
    <rect x="26" y="4" width="16" height="8" rx="3" fill="#dc836c" />
  </g>
);

const poster = () => (
  <g>
    <rect x="0" y="0" width="26" height="20" rx="1" fill="#f5efe3" stroke="#c9bda6" />
    <text x="13" y="13" textAnchor="middle" fontSize="9" fill="#3d3a33" fontFamily="ui-monospace, monospace">
      O(n)
    </text>
  </g>
);

const lamp = () => (
  <g>
    <path d="M2 0 H18 L22 12 H-2 Z" fill="#f2c85c" />
    <rect x="9" y="12" width="2" height="22" fill="#8a8578" />
    <ellipse cx="10" cy="34" rx="8" ry="2.5" fill="#8a8578" />
    <circle cx="10" cy="14" r="14" fill="#ffe9a8" opacity="0.28" />
  </g>
);

const bookshelf = () => (
  <g>
    <rect x="0" y="-6" width="34" height="40" rx="2" fill="#7a5433" />
    <rect x="3" y="-3" width="28" height="12" fill="#5f402685" />
    <rect x="3" y="12" width="28" height="12" fill="#5f402685" />
    {[0, 1, 2, 3].map((i) => (
      <rect key={i} x={5 + i * 6} y={-2} width="4" height="10" fill={["#c05c5c", "#5c86c0", "#c0a75c", "#6ec05c"][i]} />
    ))}
    {[0, 1, 2].map((i) => (
      <rect key={i} x={5 + i * 6} y={13} width="4" height="10" fill={["#8c5cc0", "#c05c95", "#5cc0b4"][i]} />
    ))}
  </g>
);

const pc = () => (
  <g>
    <rect x="0" y="0" width="18" height="30" rx="2" fill="#23262e" stroke="#3b4150" />
    <rect x="3" y="4" width="12" height="3" rx="1" fill="#ff5f8f" />
    <rect x="3" y="9" width="12" height="3" rx="1" fill="#5fd0ff" />
    <rect x="3" y="14" width="12" height="3" rx="1" fill="#9d5fff" />
    <circle cx="9" cy="24" r="3" fill="#2f3542" stroke="#59627f" />
  </g>
);

const cat = () => (
  <g>
    <ellipse cx="14" cy="18" rx="13" ry="8" fill="#5b5b64" />
    <circle cx="25" cy="12" r="7" fill="#5b5b64" />
    <path d="M20 7 L21 1 L25 5 Z M30 7 L29 1 L25 5 Z" fill="#5b5b64" />
    <circle cx="23" cy="11" r="1.2" fill="#ffd166" />
    <circle cx="28" cy="11" r="1.2" fill="#ffd166" />
    <path d="M1 18 Q-6 12 0 8" stroke="#5b5b64" strokeWidth="3.5" fill="none" strokeLinecap="round" />
  </g>
);

const dog = () => (
  <g>
    <ellipse cx="14" cy="18" rx="13" ry="8.5" fill="#b98a5a" />
    <circle cx="26" cy="13" r="7.5" fill="#b98a5a" />
    <ellipse cx="21" cy="8" rx="3" ry="5" fill="#9c7146" transform="rotate(-20 21 8)" />
    <ellipse cx="31" cy="8" rx="3" ry="5" fill="#9c7146" transform="rotate(20 31 8)" />
    <circle cx="24" cy="12" r="1.2" fill="#2a1f14" />
    <circle cx="29" cy="12" r="1.2" fill="#2a1f14" />
    <circle cx="26.5" cy="16" r="1.8" fill="#2a1f14" />
    <path d="M1 16 Q-5 8 2 6" stroke="#b98a5a" strokeWidth="4" fill="none" strokeLinecap="round" />
  </g>
);

const rug = () => (
  <g>
    <ellipse cx="34" cy="8" rx="34" ry="8" fill="#a8556b" opacity="0.75" />
    <ellipse cx="34" cy="8" rx="22" ry="5" fill="#c9738a" opacity="0.8" />
  </g>
);

const windowView = () => (
  <g>
    <rect x="0" y="0" width="42" height="32" rx="2" fill="#8fd0f0" stroke="#e8e2d6" strokeWidth="3" />
    <circle cx="31" cy="9" r="5" fill="#fff3bf" />
    <path d="M0 24 L12 14 L22 24 L32 17 L42 26 V32 H0 Z" fill="#6aa86f" />
    <path d="M21 0 V32 M0 16 H42" stroke="#e8e2d6" strokeWidth="3" />
  </g>
);

/** Room anchor + renderer for every purchasable asset. */
const PIECES: Record<string, Piece> = {
  "furniture/rug": { x: 78, y: 150, draw: rug },
  "furniture/bed": { x: 14, y: 116, draw: bed },
  "furniture/desk": { x: 112, y: 120, draw: desk },
  "furniture/couch": { x: 20, y: 132, draw: couch },
  "furniture/bookshelf": { x: 150, y: 108, draw: bookshelf },
  "furniture/plant": { x: 166, y: 126, draw: plant },
  "furniture/poster": { x: 44, y: 44, draw: poster },
  "furniture/lamp": { x: 8, y: 110, draw: lamp },
  "furniture/pc": { x: 130, y: 96, draw: pc },
  "furniture/window": { x: 108, y: 38, draw: windowView },
  "pets/cat": { x: 92, y: 134, draw: cat },
  "pets/dog": { x: 88, y: 132, draw: dog },
};

/** Draw order: rugs and wall art behind, pets in front. */
const Z: string[] = [
  "furniture/window", "furniture/poster", "furniture/rug", "furniture/bookshelf",
  "furniture/bed", "furniture/couch", "furniture/desk", "furniture/pc",
  "furniture/lamp", "furniture/plant", "pets/cat", "pets/dog",
];

export function renderFurniture(assetPaths: string[]): JSX.Element[] {
  const owned = new Set(assetPaths);
  return Z.filter((k) => owned.has(k)).map((key) => {
    const piece = PIECES[key];
    return (
      <g key={key} transform={`translate(${piece.x} ${piece.y})`} data-testid={`furniture-${key}`}>
        {piece.draw()}
      </g>
    );
  });
}

export const KNOWN_ASSETS = Object.keys(PIECES);
