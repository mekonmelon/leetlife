/** Walkable tile graph. Floors connect only through the staircase. */
export interface Point3 { x: number; y: number; z: number }
export const SPOTS = {
  tea: { x: 4.8, y: 7.05, z: 0 }, fire: { x: 1.7, y: 5.3, z: 0 },
  hearthAisle: { x: 1.6, y: 8.2, z: 0 },
  desk: { x: 7.4, y: 6.1, z: 0 }, hall: { x: 6.2, y: 7.8, z: 0 },
  bottom: { x: 10.7, y: 8.5, z: 0 },
  ...Object.fromEntries(Array.from({length: 12}, (_, i) => [`step${i + 1}`, {x: 10.7, y: 8.5 - (i + 1) * .4, z: (i + 1) / 3}])),
  landing: { x: 9.1, y: 3.3, z: 4 }, loft: { x: 5.5, y: 3.3, z: 4 },
  bed: { x: 2.3, y: 1.6, z: 4 }, books: { x: 5.5, y: 1.4, z: 4 },
} satisfies Record<string, Point3>;
export const LINKS: [string, string][] = [
  ["tea", "hall"], ["fire", "hearthAisle"], ["hearthAisle", "hall"], ["hall", "desk"], ["hall", "bottom"],
  ["bottom", "step1"], ...Array.from({length: 11}, (_, i):[string,string] => [`step${i+1}`, `step${i+2}`]),
  ["step12", "landing"], ["landing", "loft"], ["loft", "bed"], ["loft", "books"],
];
export const ACTIVITIES: Record<string, string> = {tea:"Making a cup of tea", fire:"Warming tiny toes", desk:"Practising a little magic", hall:"Sweeping the floor", bed:"Dreaming in the loft", books:"Reading a favourite book", landing:"Watching the stars"};
export function route(from: string, to: string): string[] {
  const queue = [[from]];
  const seen = new Set([from]);
  while (queue.length) {
    const path = queue.shift()!;
    const last = path.at(-1)!;
    if (last === to) return path.slice(1);
    for (const [a,b] of LINKS) {
      const next = a === last ? b : b === last ? a : null;
      if (next && !seen.has(next)) {seen.add(next); queue.push([...path,next]);}
    }
  }
  return [];
}
export interface Resident { position:Point3; node:string; path:string[]; target:string; rest:number; phase:number; moving:boolean; facing:number }
export function createResident(random = Math.random): Resident {
  const keys = Object.keys(ACTIVITIES);
  const node = keys[Math.floor(random()*keys.length)];
  return {position:{...SPOTS[node as keyof typeof SPOTS]}, node, path:[], target:node, rest:2, phase:0, moving:false, facing:1};
}
export function sendResident(resident:Resident, target:string) {
  // Complete the current tile segment before following a new route.
  const next = resident.path[0];
  resident.path = next ? [next, ...route(next,target)] : route(resident.node,target);
  resident.target=target; resident.rest=0;
}
export function advanceResident(resident:Resident, dt:number, night:boolean, random=Math.random) {
  resident.phase+=dt;
  if (!resident.path.length) {
    resident.moving=false;
    resident.rest-=dt;
    if (resident.rest>0) return;
    const choices = Object.keys(ACTIVITIES).filter(key=>key!==resident.node);
    const target = night && resident.node!=="bed" ? "bed" : night ? "bed" : choices[Math.floor(random()*choices.length)];
    resident.target=target;resident.path=route(resident.node,target);
    if (!resident.path.length) {resident.rest=20; return;}
  }
  resident.moving=true;
  const next=resident.path[0];
  const p=SPOTS[next as keyof typeof SPOTS];
  const dx=p.x-resident.position.x, dy=p.y-resident.position.y, dz=p.z-resident.position.z;
  const distance=Math.hypot(dx,dy,dz);
  const step=Math.min(distance, dt*1.5);
  if (Math.abs(dx-dy)>.001) resident.facing=dx-dy>0?1:-1;
  if (distance<=step+.0001) {
    resident.position={...p};resident.node=next;resident.path.shift();
    if (!resident.path.length) {resident.moving=false;resident.rest=next==="bed"?24:5+random()*7;}
  } else resident.position={x:resident.position.x+dx/distance*step,y:resident.position.y+dy/distance*step,z:resident.position.z+dz/distance*step};
}
