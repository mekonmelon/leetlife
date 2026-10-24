import type { CSSProperties } from "react";

const paths: Record<string, string> = {
  lock: "M7 10V7a5 5 0 0 1 10 0v3M5 10h14v11H5Zm7 5v3",
  quests: "M5 3h14v18H5ZM9 7h6m-6 5h6m-6 5h4",
  coin: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm0 4v10m3-8h-4a2 2 0 0 0 0 4h2a2 2 0 0 1 0 4H9",
  flame: "M13 2c2 6-3 7-3 10-2 0-3-2-3-3-6 7-2 13 5 13 9 0 12-11 1-20Zm-1 12c-4 4-2 6 0 6s4-3 0-6",
  moon: "M19 15A8 8 0 0 1 9 4a9 9 0 1 0 10 11Z",
  star: "m12 2 2.7 6.6L22 9l-5.6 5 1.6 8-6-4-6 4 1.6-8L2 9l7.3-.4Z",
  tea: "M4 8h14v7a6 6 0 0 1-12 0V8Zm14 1h2a3 3 0 0 1 0 6h-2M3 22h18M8 2v3m5-3v3",
  energy: "m13 2-8 12h6l-1 8 9-13h-7Z",
  brush: "m14 3 7 7-9 9-7-7ZM5 12c-6 0-1 7-4 10 8 2 12-4 8-7",
  minimize: "M5 16h14",
  expand: "M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5",
  home: "m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1Z",
  solve: "m8 7-5 5 5 5m8-10 5 5-5 5m-3-13-2 20",
  shop: "M3 8h18l-2-5H5Zm1 0v13h16V8M9 21v-8h6v8M3 8a3 3 0 0 0 6 0 3 3 0 0 0 6 0 3 3 0 0 0 6 0",
  stats: "M4 20V10m8 10V4m8 16v-7",
  settings: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1Z",
  leaf: "M20 3C7 2 2 8 5 15c6 9 17 0 15-12ZM4 21 16 9",
  arrow: "M5 12h14m-5-5 5 5-5 5",
  check: "m5 12 4 4L19 6",
  close: "m6 6 12 12M6 18 18 6",
  heart: "M20 5c-3-3-7-1-8 1C8-1 0 5 4 11l8 9 8-9c2-2 2-4 0-6Z",
};

export function Icon({ name, size = 18, style }: { name: string; size?: number; style?: CSSProperties }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" style={style}><path d={paths[name] ?? paths.leaf} /></svg>;
}
