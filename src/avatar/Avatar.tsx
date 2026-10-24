// The character. Pose and expression are derived from backend state only.
import type { AvatarState } from "../api/types";

export type Pose = "idle" | "eating" | "celebrating" | "sleepy" | "sad";

export function poseFor(avatar: AvatarState, hour: number, justRewarded: boolean): Pose {
  if (justRewarded) return "celebrating";
  if (avatar.hunger < 30) return "eating";
  if (avatar.energy < 30 || hour >= 23 || hour < 6) return "sleepy";
  if (avatar.happiness < 40) return "sad";
  return "idle";
}

const SKIN = "#f0c49b";
const HAIR = "#3b2f2a";

export function Avatar({ avatar, pose, wearing }: { avatar: AvatarState; pose: Pose; wearing: string[] }) {
  const hoodie = wearing.includes("outfits/hoodie");
  const headphones = wearing.includes("outfits/headphones");
  const shirt = hoodie ? "#4a5a8a" : "#5d8aa8";
  const armLift = pose === "celebrating" ? -14 : 0;
  const eyeY = pose === "sleepy" ? 0 : -1;

  return (
    <g data-testid="avatar" data-pose={pose} transform="translate(72 84)">
      {pose === "celebrating" && (
        <g data-testid="confetti">
          {[-18, -6, 8, 20].map((x, i) => (
            <rect key={i} x={x} y={-34 - (i % 2) * 6} width="3" height="6" rx="1" fill={["#ffd166", "#ef7674", "#6ec7a8", "#7ea6ff"][i]} transform={`rotate(${i * 27} ${x} ${-32})`} />
          ))}
        </g>
      )}

      {/* legs */}
      <rect x="-8" y="16" width="6" height="16" rx="2.5" fill="#39415c" />
      <rect x="2" y="16" width="6" height="16" rx="2.5" fill="#39415c" />
      {/* body */}
      <rect x="-11" y="-4" width="22" height="22" rx="7" fill={shirt} />
      {hoodie && <path d="M-11 2 Q0 8 11 2" stroke="#36446b" strokeWidth="2" fill="none" />}
      {/* arms */}
      <rect x="-17" y={-2 + armLift} width="6" height="16" rx="3" fill={shirt} transform={pose === "celebrating" ? "rotate(-28 -14 2)" : undefined} />
      <rect x="11" y={-2 + armLift} width="6" height="16" rx="3" fill={shirt} transform={pose === "celebrating" ? "rotate(28 14 2)" : undefined} />
      {/* head */}
      <circle cx="0" cy="-16" r="13" fill={SKIN} />
      <path d="M-13 -19 Q0 -33 13 -19 Q8 -26 0 -26 Q-8 -26 -13 -19 Z" fill={HAIR} />
      {headphones && (
        <g>
          <path d="M-14 -18 Q0 -34 14 -18" stroke="#2b2f3a" strokeWidth="3" fill="none" />
          <rect x="-18" y="-20" width="6" height="9" rx="2.5" fill="#2b2f3a" />
          <rect x="12" y="-20" width="6" height="9" rx="2.5" fill="#2b2f3a" />
        </g>
      )}
      {/* face */}
      {pose === "sleepy" ? (
        <>
          <path d="M-7 -16 q3 2 6 0" stroke="#2a2118" strokeWidth="1.6" fill="none" strokeLinecap="round" />
          <path d="M1 -16 q3 2 6 0" stroke="#2a2118" strokeWidth="1.6" fill="none" strokeLinecap="round" />
          <text x="14" y="-26" fontSize="9" fill="#8b93a7">z</text>
        </>
      ) : (
        <>
          <circle cx="-4.5" cy={-17 + eyeY} r="1.7" fill="#2a2118" />
          <circle cx="4.5" cy={-17 + eyeY} r="1.7" fill="#2a2118" />
        </>
      )}
      {pose === "sad" ? (
        <path d="M-5 -9 q5 -4 10 0" stroke="#2a2118" strokeWidth="1.6" fill="none" strokeLinecap="round" />
      ) : pose === "eating" ? (
        <ellipse cx="0" cy="-10" rx="3.5" ry="3" fill="#8c4a3f" />
      ) : (
        <path d="M-5 -11 q5 4 10 0" stroke="#2a2118" strokeWidth="1.6" fill="none" strokeLinecap="round" />
      )}
      {pose === "eating" && (
        <g data-testid="food" transform="translate(16 -6)">
          <ellipse cx="0" cy="0" rx="7" ry="3" fill="#f5efe3" />
          <circle cx="0" cy="-2" r="3" fill="#ffd166" />
        </g>
      )}
      {/* level badge */}
      <g transform="translate(-30 -30)">
        <circle r="9" fill="#2c3350" stroke="#59627f" />
        <text textAnchor="middle" y="3.5" fontSize="9" fill="#ffd166" fontFamily="ui-monospace, monospace">
          {avatar.level}
        </text>
      </g>
    </g>
  );
}
