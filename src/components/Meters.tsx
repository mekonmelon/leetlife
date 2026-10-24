import { Icon } from "./Icon";
import type { AvatarState } from "../api/types";

const METERS: { key: keyof Pick<AvatarState, "hunger" | "energy" | "happiness">; label: string; icon: string }[] = [
  { key: "hunger", label: "Fed", icon: "tea" },
  { key: "energy", label: "Energy", icon: "energy" },
  { key: "happiness", label: "Mood", icon: "heart" },
];

export function Meters({ avatar }: { avatar: AvatarState }) {
  return (
    <div className="meters" data-testid="meters">
      {METERS.map(({ key, label, icon }) => {
        const value = avatar[key];
        return (
          <div className="meter" key={key} title={`${label}: ${value}/100`}>
            <span className="meter-icon" aria-hidden><Icon name={icon} size={13}/></span>
            <span className="meter-name">{label}</span>
            <div className="meter-track">
              <div
                className="meter-fill"
                data-testid={`meter-${key}`}
                data-value={value}
                style={{ width: `${value}%` }}
                data-low={value < 30 ? "true" : undefined}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function XpBar({ into, needed, level }: { into: number; needed: number; level: number }) {
  const pct = needed > 0 ? Math.min(100, Math.round((into / needed) * 100)) : 0;
  return (
    <div className="xpbar" title={`${into} / ${needed} XP to level ${level + 1}`} data-testid="xpbar" data-pct={pct}>
      <div className="xpbar-fill" style={{ width: `${pct}%` }} />
      <span className="xpbar-label">
        {into}/{needed} XP · next stop, level {level + 1}
      </span>
    </div>
  );
}
