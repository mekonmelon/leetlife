import { useEffect, useRef, useState } from "react";
import { api, on } from "../api/client";
import type { Placement, RoomId, Snapshot } from "../api/types";
import { Icon } from "../components/Icon";
import { ACTIVITIES, SPOTS, advanceResident, createResident, sendResident } from "./life";
import { ROOMS, WALLPAPERS, canPlace, defaultPlacement, isWallArt } from "./layout";
import { FLOWERS, H, W, drawGarden, drawHouse, flowerStage, project, unproject } from "../game/pixels";

export function House({ snapshot, hour, justRewarded: _reward, editing=false, onChanged, onError, onShop, onFinish }: {
  snapshot:Snapshot; hour:number; justRewarded:boolean; editing?:boolean;
  onChanged?:(s:Snapshot,notice?:string)=>void; onError?:(message:string)=>void; onShop?:()=>void; onFinish?:()=>void;
}) {
  const canvas=useRef<HTMLCanvasElement>(null);
  const resident=useRef(createResident());
  const current=useRef({snapshot,hour,editing});current.current={snapshot,hour,editing};
  const [area,setArea]=useState<"house"|"garden">("house");
  const [locked,setLocked]=useState(false);
  const [status,setStatus]=useState(ACTIVITIES[resident.current.node]);
  const [room,setRoom]=useState<RoomId>("hearth");
  const [selected,setSelected]=useState("");
  const [busy,setBusy]=useState(false);
  const [plot,setPlot]=useState<number|null>(null);
  const [species,setSpecies]=useState("daisy");
  const [now,setNow]=useState(Math.floor(Date.now()/1000));
  const unlocked=snapshot.avatar.level>=3;
  const inventory=snapshot.inventory.filter(o=>canPlace(o.item));
  const selectedItem=inventory.find(o=>String(o.item.id)===selected)?.item;
  const fail=(error:unknown)=>onError?.(String(error));

  useEffect(()=>{
    let live=true,unlisten:(()=>void)|undefined;
    void on("home-visited",()=>{resident.current=createResident();setArea("house");setLocked(false);onFinish?.();}).then(fn=>{if(live)unlisten=fn;else fn();}).catch(fail);
    return()=>{live=false;unlisten?.();};
  },[]);
  useEffect(()=>{if(editing){setArea("house");setLocked(false);}},[editing]);
  useEffect(()=>{
    const timer=setInterval(()=>{const r=resident.current;setStatus(r.moving?(r.position.z>0&&r.position.z<4?"Climbing the stairs":"Taking a little walk"):ACTIVITIES[r.node]??"Taking a little walk");setNow(Math.floor(Date.now()/1000));},500);
    return()=>clearInterval(timer);
  },[]);
  useEffect(()=>{
    if(typeof CanvasRenderingContext2D==="undefined")return;
    const context=canvas.current?.getContext("2d",{alpha:false});if(!context)return;
    context.imageSmoothingEnabled=false;
    let frame=0,last=0,elapsed=0;
    const reduced=window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const render=(time:number)=>{
      const dt=last?Math.min((time-last)/1000,.08):0;last=time;elapsed+=dt;
      const state=current.current,night=state.hour>=20||state.hour<6;
      if(!document.hidden){
        if(!state.editing&&!reduced&&area==="house")advanceResident(resident.current,dt,night);
        if(area==="house")drawHouse(context,state.snapshot,resident.current,elapsed,night,state.editing);
        else drawGarden(context,state.snapshot,elapsed,night,Math.floor(Date.now()/1000));
      }
      frame=requestAnimationFrame(render);
    };
    frame=requestAnimationFrame(render);return()=>cancelAnimationFrame(frame);
  },[area]);

  async function place(placement:Placement) {
    if(!selectedItem||busy)return;setBusy(true);
    try{onChanged?.(await api.placeItem(selectedItem.id,placement),"Placed in your house.");}catch(e){fail(e);}finally{setBusy(false);}
  }
  function pointer(event:React.MouseEvent<HTMLCanvasElement>) {
    const b=event.currentTarget.getBoundingClientRect();
    const scale=Math.min(b.width/W,b.height/H),x=(event.clientX-b.left-(b.width-W*scale)/2)/scale,y=(event.clientY-b.top-(b.height-H*scale)/2)/scale;
    if(area==="garden"){
      let nearest=-1,distance=38;
      for(let i=0;i<6;i++){const p=project(3+i%3*2.4,3+Math.floor(i/3)*2.7);const d=Math.hypot(x-p[0],y-p[1]);if(d<distance){nearest=i;distance=d;}}
      if(nearest>=0)setPlot(nearest);return;
    }
    if(editing){
      if(!selectedItem){onError?.("Choose a treasure to place first.");return;}
      const p=unproject(x,y,room==="loft"?4:0),left=room==="loft"?1:room==="studio"?6.2:1,span=room==="loft"?8:room==="studio"?3:4;
      const placement={room,x:Math.round(Math.max(30,Math.min(290,30+(p.x-left)/span*260))),y:isWallArt(selectedItem.asset_path)?90:Math.round(Math.max(155,Math.min(210,155+(p.y-(room==="loft"?.8:5.1))/2.4*60)))};
      void place(placement);return;
    }
    const keys=Object.keys(ACTIVITIES);
    const target=keys.sort((a,b)=>{const pa=SPOTS[a as keyof typeof SPOTS],pb=SPOTS[b as keyof typeof SPOTS];const aa=project(pa.x,pa.y,pa.z),bb=project(pb.x,pb.y,pb.z);return Math.hypot(x-aa[0],y-aa[1])-Math.hypot(x-bb[0],y-bb[1]);})[0];
    sendResident(resident.current,target);
  }
  async function gardenAction(action:"plant"|"water"|"harvest") {
    if(plot===null||busy)return;setBusy(true);
    try{onChanged?.(await api.gardenAction(action,plot,species),action==="plant"?"A little seed, a new beginning.":action==="water"?"A drink for your flowers.":"Flowers picked! +12 coins.");}catch(e){fail(e);}finally{setBusy(false);}
  }
  const flower=plot===null?undefined:snapshot.home?.garden?.[plot];
  return <section className="pixel-world" data-testid="house" data-room-level={snapshot.room_level} data-room={resident.current.position.z>=4?"loft":"hearth"}>
    <canvas ref={canvas} width={W} height={H} className="world-canvas" onClick={pointer} role="img" aria-label={area==="house"?"A pixel art cutaway house with a living room, study, sleeping loft and walkable staircase":"Your backyard flower garden with six planting beds"}/>
    <div className="world-switch" aria-label="Places"><button aria-pressed={area==="house"} onClick={()=>{setArea("house");setPlot(null);}}><Icon name="home" size={14}/>House</button><button aria-pressed={area==="garden"} onClick={()=>{if(unlocked){setArea("garden");onFinish?.();}else setLocked(true);}}><Icon name={unlocked?"leaf":"lock"} size={14}/>{unlocked?"Backyard":"Backyard · LV 3"}</button></div>
    {!editing&&area==="house"&&<div className="resident-caption"><i/>{status}<span>Click a spot to send Susuwatari there</span></div>}
    {area==="garden"&&<div className="garden-controls game-popup"><div><strong>The little backyard</strong><span>Plant, water, and let your flowers grow.</span></div><div className="garden-beds" aria-label="Flower beds">{Array.from({length:6},(_,i)=><button key={i} aria-pressed={plot===i} aria-label={`Flower bed ${i+1}${snapshot.home?.garden?.[i]?", planted":", empty"}`} onClick={()=>setPlot(i)}>{i+1}<i style={{background:snapshot.home?.garden?.[i]?FLOWERS.find(f=>f.id===snapshot.home?.garden?.[i].species)?.color:"#876e65"}}/></button>)}</div>
      {plot===null?<p>Choose a bed in the garden.</p>:!flower?<><label>Seed pouch<select aria-label="Flower seeds" value={species} onChange={e=>setSpecies(e.target.value)}>{FLOWERS.map(f=><option key={f.id} value={f.id}>{f.name}</option>)}</select></label><button className="primary" disabled={busy||snapshot.avatar.coins<8} onClick={()=>void gardenAction("plant")}>Plant in bed {plot+1} · 8 coins</button></>:<><p>{FLOWERS.find(f=>f.id===flower.species)?.name} · {flower.watered_at===null?"Needs a little water":flowerStage(flower,now)===3?"In full bloom":`Growing · ${Math.max(1,Math.ceil((1800-now+flower.planted_at)/60))} min to bloom`}</p><button disabled={busy||(flower.watered_at!==null&&flowerStage(flower,now)<3)} onClick={()=>void gardenAction(flower.watered_at===null?"water":"harvest")}>{flower.watered_at===null?"Water flowers":flowerStage(flower,now)===3?"Pick flowers · +12 coins":"Growing quietly…"}</button></>}
      <small>Blooms in 30 minutes after planting, once watered. Grows while the app is closed.</small>
    </div>}
    {editing&&<div className="decorate-controls game-popup"><div className="popup-heading"><strong>Make yourself at home</strong><button onClick={onFinish} aria-label="Finish decorating"><Icon name="check" size={14}/>Done</button></div><label>Part of the house<select aria-label="Part of the house" value={room} onChange={e=>setRoom(e.target.value as RoomId)}>{ROOMS.map(r=><option key={r.id} value={r.id}>{r.name}</option>)}</select></label><label>Bring a treasure<select value={selected} onChange={e=>setSelected(e.target.value)}><option value="">Choose from your collection</option>{inventory.map(o=><option key={o.item.id} value={o.item.id}>{o.item.name}</option>)}</select></label><div className="popup-actions"><button disabled={!selectedItem||busy} onClick={()=>selectedItem&&void place({...defaultPlacement(selectedItem),room})}>Place here</button><button onClick={onShop}>Find more treasures</button></div><small>Select a treasure, then click a tile to move it. Stairs stay clear.</small><div className="wallpaper-swatches">{WALLPAPERS.filter(w=>w.asset==="wallpapers/linen"||snapshot.inventory.some(o=>o.item.asset_path===w.asset)).map(w=><button key={w.asset} title={w.name} style={{background:w.color}} disabled={busy} aria-label={`Use ${w.name} wallpaper`} onClick={()=>{setBusy(true);void api.setWallpaper(room,w.asset).then(s=>onChanged?.(s)).catch(fail).finally(()=>setBusy(false));}}/>)}</div></div>}
    {locked&&<div className="unlock-note game-popup" role="dialog" aria-label="Backyard unlock"><Icon name="lock" size={24}/><h2>A gate to something lovely.</h2><p>Reach level 3 to unlock your backyard. Plant flowers, give them a little water, and pick the blooms for coins.</p><p>You’re at level {snapshot.avatar.level}. Every problem brings you closer.</p><button onClick={()=>setLocked(false)}>Keep growing</button></div>}
    <ul className="sr-only" aria-label="Placed furniture">{inventory.filter(o=>o.equipped).map(o=><li key={o.item.id} data-testid={`furniture-${o.item.asset_path}`}>{o.item.name} in {snapshot.home?.placements[o.item.id]?.room??defaultPlacement(o.item).room}</li>)}</ul>
  </section>;
}
