import { useId } from "react";
import type { AvatarState } from "../api/types";
export type Pose = "idle" | "eating" | "celebrating" | "sleepy" | "sad";
export function poseFor(avatar: AvatarState, hour: number, justRewarded: boolean): Pose {
  if (justRewarded) return "celebrating";
  if (avatar.hunger < 30) return "eating";
  if (avatar.energy < 30 || hour >= 23 || hour < 6) return "sleepy";
  if (avatar.happiness < 40) return "sad";
  return "idle";
}

/** Susuwatari: the fuzzy black soot sprite, with its familiar oversized eyes. */
export function SpiritDrawing({ pose = "idle", wearing = [] }: { pose?: Pose; wearing?: string[] }) {
  const id=useId().replace(/:/g, "");
  const sleeping=pose==="sleepy";
  const fur=Array.from({length:76},(_,i)=>{
    const angle=i*Math.PI*2/76;
    const inner=22+Math.sin(i*2.7)*1.8;
    const outer=27+Math.sin(i*1.9)*3.3;
    return <path key={i} d={`M${Math.cos(angle)*inner} ${-28+Math.sin(angle)*inner}l${Math.cos(angle)*(outer-inner)} ${Math.sin(angle)*(outer-inner)}`} />;
  });
  return <g className={`spirit-art spirit-${pose}`}>
    <defs><radialGradient id={`${id}-soot`} cx="35%" cy="28%" r="80%"><stop stopColor="#3d393d"/><stop offset=".65" stopColor="#252228"/><stop offset="1" stopColor="#15141a"/></radialGradient></defs>
    <ellipse cx="0" cy="2" rx="22" ry="4" fill="#252127" opacity=".2"/>
    <g className="spirit-body">
      <path className="spirit-foot left" d="m-8-9-4 10-4 1m3-1 1 3" stroke="#2b252c" strokeWidth="1.3" fill="none" strokeLinecap="round"/>
      <path className="spirit-foot right" d="m8-9 4 10 4 1m-3-1-1 3" stroke="#2b252c" strokeWidth="1.3" fill="none" strokeLinecap="round"/>
      <g stroke="#29252c" strokeWidth="1.1" strokeLinecap="round" fill="none">{fur}</g>
      <path d="M-24-28q-3-17 13-23 15-7 28 5 12 12 5 27-6 15-22 15-20-1-24-24Z" fill={`url(#${id}-soot)`}/>
      {sleeping?<g fill="#f6f1df"><path d="M-17-28q8 7 15-1-6 12-15 1Z"/><path d="M1-29q9 7 16-1-7 12-16 1Z"/></g>:<g className="spirit-eyes"><ellipse cx="-8.5" cy="-29" rx="8.2" ry="9.2" fill="#f9f6e8"/><ellipse cx="8.8" cy="-30.4" rx="8.1" ry="9" fill="#f9f6e8"/><circle cx={pose==="sad"?-7:-7.5} cy={pose==="sad"?-27:-30} r="2.7" fill="#17151a"/><circle cx="8.2" cy={pose==="sad"?-28:-31.2} r="2.7" fill="#17151a"/></g>}
      {wearing.includes("outfits/hoodie")&&<path d="M-20-12q19 9 40-1l-1 5q-19 7-37 0Z" fill="#b5a7bf"/>}
      {wearing.includes("outfits/headphones")&&<g fill="none" stroke="#a99ab2" strokeWidth="2"><path d="M-25-31q0-34 50 0"/><path d="M-26-33v11m52-11v11" strokeWidth="4"/></g>}
      {pose==="eating"&&<g data-testid="food"><path d="m-22-19-10 5 5 6m48-11 10 5-5 6" stroke="#211e25" strokeWidth="1.3" fill="none"/><path d="m0-17 2 5 5-1-2 5 3 4-5 1-2 5-4-4-5 2 1-6-4-3 5-3 1-5Z" fill="#e7c1a1" stroke="#d2a68f" strokeWidth=".5"/></g>}
      {pose==="celebrating"&&<g data-testid="confetti">{[-27,25,-15,16].map((x,i)=><path key={x} d={`M${x} ${-47-i%2*12}l2-4 2 4 4 2-4 2-2 4-2-4-4-2Z`} fill={["#e7c1a1","#b8cbaa","#e6d496","#c6b7d0"][i]}/>)}</g>}
      {sleeping&&<g className="sleep-bubbles" fill="none" stroke="#b0a68b" strokeWidth=".8"><circle cx="29" cy="-42" r="2"/><circle cx="34" cy="-50" r="1.4"/><path d="m33-57 4-4h-4m0 4h4"/></g>}
    </g>
  </g>;
}
export function CreaturePortrait({ pose="idle",className="" }: {pose?:Pose;className?:string}) {
  return <svg className={`creature-portrait ${className}`} viewBox="-44 -68 88 79" role="img" aria-label={`Susuwatari, your ${pose==="sleepy"?"sleeping ":""}soot sprite`}><SpiritDrawing pose={pose}/></svg>;
}
export function Avatar({avatar,pose,wearing}:{avatar:AvatarState;pose:Pose;wearing:string[]}) {
  return <g data-testid="avatar" data-pose={pose} aria-label={`Susuwatari, level ${avatar.level}`}><SpiritDrawing pose={pose} wearing={wearing}/></g>;
}
