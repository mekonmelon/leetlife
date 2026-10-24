// The room: walls scale with the room upgrade level, furniture comes from the
// inventory, and the speech bubble text is computed by Rust.
import { Avatar, poseFor } from "../avatar/Avatar";
import { renderFurniture } from "./Furniture";
import type { Snapshot } from "../api/types";

const WALLS = [
  { wall: "#2b3350", floor: "#4a3b2f", trim: "#59627f", name: "Studio flat" },
  { wall: "#31406b", floor: "#5b4636", trim: "#6d7ba3", name: "Studio" },
  { wall: "#3a3350", floor: "#6b4f3a", trim: "#8a6fa8", name: "Loft" },
];

export function House({ snapshot, hour, justRewarded }: { snapshot: Snapshot; hour: number; justRewarded: boolean }) {
  const theme = WALLS[Math.min(snapshot.room_level, WALLS.length - 1)];
  const equipped = snapshot.inventory.filter((o) => o.equipped).map((o) => o.item.asset_path);
  const pose = poseFor(snapshot.avatar, hour, justRewarded);
  const night = hour >= 20 || hour < 6;

  return (
    <div className="house" data-testid="house" data-room-level={snapshot.room_level}>
      <svg viewBox="0 0 220 180" role="img" aria-label={`${theme.name}, level ${snapshot.avatar.level} avatar`}>
        <defs>
          <linearGradient id="wallGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={theme.wall} />
            <stop offset="100%" stopColor={theme.wall} stopOpacity="0.72" />
          </linearGradient>
          <radialGradient id="glow" cx="50%" cy="40%" r="60%">
            <stop offset="0%" stopColor="#ffe9a8" stopOpacity={night ? 0.16 : 0.06} />
            <stop offset="100%" stopColor="#ffe9a8" stopOpacity="0" />
          </radialGradient>
        </defs>

        <rect x="0" y="0" width="220" height="150" fill="url(#wallGrad)" />
        <rect x="0" y="150" width="220" height="30" fill={theme.floor} />
        <rect x="0" y="147" width="220" height="4" fill={theme.trim} />
        <rect x="0" y="0" width="220" height="180" fill="url(#glow)" />

        {renderFurniture(equipped)}
        <Avatar avatar={snapshot.avatar} pose={pose} wearing={equipped} />
      </svg>

      <div className="speech" data-testid="mood">
        {snapshot.mood}
      </div>
    </div>
  );
}
