import { describe,it,expect } from "vitest";
import { SPOTS,advanceResident,createResident,route,sendResident } from "../src/house/life";
import { flowerStage,project,unproject } from "../src/game/pixels";
describe("a grounded little life",()=>{
  it("reaches the loft only through all twelve stairs",()=>{
    const path=route("tea","bed");
    for(let i=1;i<=12;i++)expect(path).toContain(`step${i}`);
    expect(path.indexOf("step1")).toBeLessThan(path.indexOf("step12"));
    expect(path.at(-1)).toBe("bed");
    expect(route("bed","tea").filter(p=>p.startsWith("step"))).toEqual(Array.from({length:12},(_,i)=>`step${12-i}`));
  });
  it("walks continuously, climbs the staircase, and rests at the bed",()=>{
    const r=createResident(()=>0);sendResident(r,"bed");
    let previous={...r.position},climbed=false;
    for(let i=0;i<1000&&r.node!=="bed";i++){
      advanceResident(r,.05,false,()=>0);
      expect(Math.hypot(r.position.x-previous.x,r.position.y-previous.y,r.position.z-previous.z)).toBeLessThanOrEqual(.076);
      if(r.position.z>0&&r.position.z<4){climbed=true;expect(r.position.x).toBeCloseTo(10.7);}
      previous={...r.position};
    }
    expect(climbed).toBe(true);expect(r.node).toBe("bed");expect(r.moving).toBe(false);expect(r.position).toEqual(SPOTS.bed);
    for(let i=0;i<1000;i++)advanceResident(r,.05,true,()=>0);
    expect(r.node).toBe("bed");expect(r.path).toEqual([]);
  });
  it("can change plans mid-walk without jumping to another tile",()=>{
    const r=createResident(()=>0);sendResident(r,"bed");advanceResident(r,.2,false);
    const before={...r.position};sendResident(r,"desk");expect(r.position).toEqual(before);
    for(let i=0;i<800&&r.node!=="desk";i++)advanceResident(r,.05,false);
    expect(r.position).toEqual(SPOTS.desk);
  });
  it("maps tile clicks back into the same floor and keeps unwatered flowers dormant",()=>{
    const p=project(4,6,4),tile=unproject(p[0],p[1],4);expect(tile).toEqual({x:4,y:6,z:4});
    expect(flowerStage({planted_at:0,watered_at:null},5000)).toBe(0);
    expect(flowerStage({planted_at:100,watered_at:200},1000)).toBe(1);
    expect(flowerStage({planted_at:100,watered_at:200},1900)).toBe(3);
  });
});
