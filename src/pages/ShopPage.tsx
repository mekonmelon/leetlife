import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import { Icon } from "../components/Icon";
import { PixelPreview as TreasurePreview } from "../game/PixelPreview";
import { ROOMS, canPlace, defaultPlacement } from "../house/layout";
import { formatCoins } from "../api/format";
import type { RoomId, ShopItem, Snapshot } from "../api/types";

type Collection = "all" | "home" | "companions" | "wallpaper";
export function ShopPage({ snapshot, onChanged, onError }: { snapshot: Snapshot; onChanged: (s: Snapshot, message?: string) => void; onError: (message: string) => void }) {
  const [items, setItems] = useState<ShopItem[]>([]);
  const [busy, setBusy] = useState<number | null>(null);
  const [collection,setCollection] = useState<Collection>("all");
  const [room,setRoom] = useState<RoomId>("hearth");
  const load=useCallback(async()=>{try{setItems(await api.getShop());}catch(error){onError(String(error));}},[onError]);
  useEffect(()=>{void load();},[load,snapshot.version]);
  async function bringHome(shop: ShopItem, buy = false) {
    setBusy(shop.item.id);
    try {
      let next=buy?await api.purchaseItem(shop.item.id):snapshot;
      // Apply the committed purchase before arranging it, so a failed placement
      // never leaves the coin balance or owned collection stale in the UI.
      if(buy)onChanged(next,`${shop.item.name} has found a home.`);
      if(shop.item.category==="wallpaper")next=await api.setWallpaper(room,shop.item.asset_path);
      else if(canPlace(shop.item))next=await api.placeItem(shop.item.id,{...defaultPlacement(shop.item),room});
      else if(shop.item.category!=="room")next=await api.setEquipped(shop.item.id,true);
      onChanged(next,buy?`${shop.item.name} has found a home.`:`A little treasure for ${ROOMS.find(r=>r.id===room)!.name.toLowerCase()}.`);
      await load();
    }catch(error){onError(String(error));}finally{setBusy(null);}
  }
  async function putAway(itemId:number) {
    setBusy(itemId);
    try{onChanged(await api.setEquipped(itemId,false));await load();}catch(error){onError(String(error));}finally{setBusy(null);}
  }
  const featured=[20,9,10,25,26,27,22,21,17,18,19,23,24];
  const shown=items.filter(s=>collection==="all"||collection==="companions"&&s.item.category==="pet"||collection==="wallpaper"&&s.item.category==="wallpaper"||collection==="home"&&!["pet","wallpaper"].includes(s.item.category)).sort((a,b)=>{const rank=(id:number)=>featured.includes(id)?featured.indexOf(id):id+30;return rank(a.item.id)-rank(b.item.id);});
  const owned=new Set(snapshot.inventory.filter(o=>o.equipped).map(o=>o.item.id));
  return <div className="page shop-page">
    <div className="shop-title"><div><span className="eyebrow">THE ODD & WONDERFUL</span><h1 className="page-title">A shop between the trees.</h1></div><span className="coins"><Icon name="coin" size={17}/>{formatCoins(snapshot.avatar.coins)}</span></div>
    <p className="page-subtitle">Small treasures. Unlikely companions. Quite a lot of magic.</p>
    <div className="shop-intro"><TreasurePreview asset="pets/dragon"/><div><h2>A home, one treasure at a time.</h2><p>Spend the coins you earn from practice. Everything you collect lives in your pixel house.</p></div></div>
    <div className="shop-filters" aria-label="Shop collections">{([["all","Everything"],["home","For the cottage"],["companions","Companions"],["wallpaper","Wallpapers"]] as const).map(([id,label])=><button key={id} aria-pressed={collection===id} onClick={()=>setCollection(id)}>{label}</button>)}</div>
    <label className="shop-room field">Finding a home in<select aria-label="Room to decorate" value={room} onChange={event=>setRoom(event.target.value as RoomId)}>{ROOMS.map(r=><option key={r.id} value={r.id}>{r.name}</option>)}</select></label>
    <ul className="shop" data-testid="shop">{shown.map(s=>{
      const inRoom=s.item.category==="wallpaper"?snapshot.home?.wallpapers[room]===s.item.asset_path:owned.has(s.item.id)&&(snapshot.home?.placements[s.item.id]?.room??defaultPlacement(s.item).room)===room;
      return <li key={s.item.id} className="shop-item" data-owned={s.owned} data-locked={!s.unlocked}>
        <div className="shop-art"><TreasurePreview asset={s.item.asset_path}/>{s.item.category==="pet"&&<span className="treasure-kind">a little companion</span>}</div>
        <div className="shop-main"><span className="shop-name">{s.item.name}</span><span className="shop-cost">{s.owned?<><Icon name="check" size={12}/>collected</>:<><Icon name="coin" size={12}/>{s.item.cost}</>}</span></div>
        <p className="shop-desc">{s.item.description}</p>
        {s.owned?s.item.category==="room"?<button disabled>Home upgraded</button>:inRoom&&s.item.category!=="wallpaper"?<button className="ghost" disabled={busy!==null} onClick={()=>void putAway(s.item.id)}>Put away</button>:<button className="ghost" disabled={busy!==null||inRoom} onClick={()=>void bringHome(s)}>{inRoom?"On your walls":s.item.category==="wallpaper"?"Use in this room":s.item.category==="outfit"?"Wear it":"Bring to this room"}</button>:!s.unlocked?<button disabled>Level {s.item.min_level} required</button>:<button className="primary" disabled={!s.affordable||busy!==null} onClick={()=>void bringHome(s,true)}>{busy===s.item.id?"Finding its way…":s.affordable?"Buy":"A few more coins"}</button>}
      </li>;
    })}</ul>
  </div>;
}
