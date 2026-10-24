import { useEffect } from "react";
import { CreaturePortrait, poseFor } from "./avatar/Avatar";
import { Icon } from "./components/Icon";
import { Meters, XpBar } from "./components/Meters";
import { useLeetLife } from "./useLeetLife";
import { api } from "./api/client";
import { formatCoins } from "./api/format";

export function QuickSummary() {
  const {snapshot, error, now, reload, setError} = useLeetLife();
  useEffect(()=>{const escape=(event:KeyboardEvent)=>{if(event.key==="Escape")void api.hideWindow();};window.addEventListener("keydown",escape);return()=>window.removeEventListener("keydown",escape);},[]);
  if(!snapshot)return <div className="summary-app"><CreaturePortrait pose="sleepy"/><p className="empty">{error??"A little spirit is waking up…"}</p>{error&&<button onClick={()=>void reload()}>Try again</button>}</div>;
  const pose=poseFor(snapshot.avatar,new Date(now*1000).getHours(),false);
  const done=snapshot.tasks.filter(task=>task.status==="done").length;
  const next=snapshot.tasks.find(task=>task.status==="pending");
  return <div className="summary-app">
    <header className="summary-header"><span><Icon name="leaf" size={13}/> A LITTLE CHECK-IN</span><button className="icon-button" aria-label="Tuck away summary" onClick={()=>void api.hideWindow()}><Icon name="close" size={13}/></button></header>
    <div className="summary-resident"><div className="summary-aura"/><CreaturePortrait pose={pose}/><div><span className="eyebrow">YOUR SOOT-SPRITE COMPANION</span><h1>Susuwatari is {pose==="sleepy"?"dreaming.":pose==="eating"?"peckish.":pose==="sad"?"a little wistful.":"right here."}</h1><p>{pose==="sleepy"?"A little soot. A little sleep.":"Small steps. A little company."}</p></div></div>
    <p className="summary-mood">{snapshot.mood}</p>
    <Meters avatar={snapshot.avatar}/><XpBar into={snapshot.xp_into_level} needed={snapshot.xp_for_next_level} level={snapshot.avatar.level}/>
    <div className="summary-progress"><span><Icon name="check" size={14}/>{done}/{snapshot.tasks.length} little quests</span><span><Icon name="coin" size={14}/>{formatCoins(snapshot.avatar.coins)}</span><span><Icon name="flame" size={14}/>{snapshot.avatar.streak} days</span></div>
    {next&&<div className="summary-next"><Icon name="star" size={15}/><div><small>A LITTLE SOMETHING FOR TODAY</small><strong>{next.title}</strong></div><span>+{next.reward_xp} XP</span></div>}
    <button className="primary summary-visit" onClick={()=>void api.openHome().catch(error=>setError(String(error)))}>Visit the cottage <Icon name="arrow" size={15}/></button>
    {error&&<p role="alert" className="hint">{error}</p>}
  </div>;
}
