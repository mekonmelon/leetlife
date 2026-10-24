import { useCallback, useEffect, useRef, useState } from "react";
import { QuickSummary } from "./QuickSummary";
import { CreaturePortrait } from "./avatar/Avatar";
import { HomePage } from "./pages/HomePage";
import { SolvePage } from "./pages/SolvePage";
import { ShopPage } from "./pages/ShopPage";
import { StatsPage } from "./pages/StatsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { House } from "./house/House";
import { Icon } from "./components/Icon";
import { Toast } from "./components/Toast";
import { useLeetLife } from "./useLeetLife";
import { formatCoins } from "./api/format";
import { api, isTauri } from "./api/client";

type Panel="solve"|"shop"|"quests"|"stats"|"settings";
const ACTIONS:{id:Panel|"decorate";label:string;icon:string;key:string}[]=[
  {id:"solve",label:"Log problem",icon:"solve",key:"1"},
  {id:"shop",label:"Shop",icon:"shop",key:"2"},
  {id:"decorate",label:"Decorate",icon:"brush",key:"3"},
  {id:"quests",label:"Quests",icon:"quests",key:"4"},
  {id:"stats",label:"Journal",icon:"stats",key:"5"},
  {id:"settings",label:"Settings",icon:"settings",key:"6"},
];
export default function App() {
  const summary=new URLSearchParams(window.location.search).get("view")==="summary";
  document.body.dataset.view=summary?"summary":"main";
  document.body.dataset.native=String(isTauri());
  return summary?<QuickSummary/>:<WorldApp/>;
}
function WorldApp() {
  const {snapshot,error,notice,now,justRewarded,applySnapshot,setError,setNotice,reload}=useLeetLife();
  const [panel,setPanel]=useState<Panel|null>(null);
  const [editing,setEditing]=useState(false);
  const dialog=useRef<HTMLDivElement>(null);
  const focusBefore=useRef<HTMLElement|null>(null);
  const onError=useCallback((m:string)=>setError(m),[setError]);
  const onNotice=useCallback((m:string)=>setNotice(m),[setNotice]);
  const open=(next:Panel)=>{setPanel(next);setEditing(false);};
  useEffect(()=>{
    if(!panel)return;
    focusBefore.current=document.activeElement as HTMLElement;
    dialog.current?.querySelector<HTMLButtonElement>("button")?.focus();
    return()=>{focusBefore.current?.focus();};
  },[panel]);
  useEffect(()=>{
    const keyboard=(e:KeyboardEvent)=>{
      if(e.key==="Escape"){
        if(panel)setPanel(null);else if(editing)setEditing(false);else if(isTauri())void api.hideWindow().catch(onError);
        return;
      }
      if(panel&&e.key==="Tab"){
        const all=dialog.current?.querySelectorAll<HTMLElement>('button:not(:disabled),input,select,a[href],[tabindex="0"]');
        if(!all?.length)return;const first=all[0],last=all[all.length-1];
        if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}
        else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}
      }
      if((e.target as HTMLElement).closest("input,textarea,select")||e.metaKey||e.ctrlKey||e.altKey)return;
      const action=ACTIONS.find(a=>a.key===e.key);
      if(action){if(action.id==="decorate"){setPanel(null);setEditing(v=>!v);}else open(action.id);}
    };
    window.addEventListener("keydown",keyboard);return()=>window.removeEventListener("keydown",keyboard);
  },[panel,editing,onError]);
  if(!snapshot)return <div className="game-app game-loading"><CreaturePortrait pose="sleepy"/><p>{error??"Lighting the little house…"}</p>{error&&<button onClick={()=>void reload()}>Try again</button>}</div>;
  const completed=snapshot.tasks.filter(t=>t.status==="done").length;
  return <div className="game-app">
    <House snapshot={snapshot} hour={new Date(now*1000).getHours()} justRewarded={justRewarded} editing={editing} onChanged={applySnapshot} onError={onError} onShop={()=>open("shop")} onFinish={()=>setEditing(false)}/>
    <div className="game-drag-bar" aria-label="Drag LeetLife window" onMouseDown={e=>{if(e.button===0&&!(e.target as HTMLElement).closest("button"))void api.startDragging().catch(onError);}}><span>LEETLIFE <span>· a little life between problems</span></span><div><button aria-label="Minimize LeetLife" title="Minimize" onClick={()=>void api.minimizeWindow().catch(onError)}><Icon name="minimize" size={15}/></button><button aria-label="Tuck away LeetLife" title="Tuck away" onClick={()=>void api.hideWindow().catch(onError)}><Icon name="close" size={15}/></button></div></div>
    <div className="game-hud"><button className="level-hud" onClick={()=>open("quests")} aria-label={`Susuwatari level ${snapshot.avatar.level}, ${snapshot.xp_into_level} of ${snapshot.xp_for_next_level} XP`}><span className="hud-soot"><CreaturePortrait/></span><span><strong>LV {snapshot.avatar.level}</strong><span className="hud-xp"><i style={{width:`${Math.min(100,snapshot.xp_into_level/snapshot.xp_for_next_level*100)}%`}}/></span></span></button><div className="hud-wallet"><span title="Coins for your cozy home"><Icon name="coin" size={15}/>{formatCoins(snapshot.avatar.coins)}</span><span title={`Best streak: ${snapshot.avatar.best_streak} days`}><Icon name="flame" size={15}/>{snapshot.avatar.streak}</span></div></div>
    <nav className="game-dock" aria-label="Game actions">{ACTIONS.map(a=><button key={a.id} title={`${a.label} (${a.key})`} aria-pressed={a.id==="decorate"?editing:panel===a.id} onClick={()=>{if(a.id==="decorate"){setPanel(null);setEditing(v=>!v);}else open(a.id);}}><span className="dock-icon"><Icon name={a.icon} size={21}/>{a.id==="quests"&&completed<snapshot.tasks.length&&<i/>}</span><span>{a.label}</span></button>)}</nav>
    {panel&&<div className="game-modal-backdrop" onMouseDown={e=>{if(e.target===e.currentTarget)setPanel(null);}}><div className="game-modal" role="dialog" aria-modal="true" aria-label={ACTIONS.find(a=>a.id===panel)?.label} ref={dialog}><header><span><Icon name={panel} size={15}/>{ACTIONS.find(a=>a.id===panel)?.label}</span><button aria-label="Return to house" onClick={()=>setPanel(null)}><Icon name="close" size={16}/></button></header><main className="modal-content">
      {panel==="solve"&&<SolvePage onChanged={applySnapshot} onError={onError}/>}
      {panel==="shop"&&<ShopPage snapshot={snapshot} onChanged={applySnapshot} onError={onError}/>}
      {panel==="quests"&&<><p className="quest-date">{snapshot.weekday} — Day {snapshot.day_number}</p><HomePage snapshot={snapshot} now={now} justRewarded={justRewarded} onChanged={applySnapshot} onError={onError} onSolve={()=>open("solve")} onShop={()=>open("shop")}/><p className="quest-mood" data-testid="mood">{snapshot.mood}</p></>}
      {panel==="stats"&&<StatsPage now={now} onError={onError}/>}
      {panel==="settings"&&<SettingsPage onError={onError} onNotice={onNotice}/>}
    </main></div></div>}
    <Toast message={error??notice} onDismiss={()=>error?setError(null):setNotice(null)}/>
  </div>;
}
