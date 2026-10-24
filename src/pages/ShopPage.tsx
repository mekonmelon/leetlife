import { useCallback, useEffect, useState } from "react";
import { api } from "../api/client";
import { formatCoins } from "../api/format";
import type { ShopItem, Snapshot } from "../api/types";

export function ShopPage({
  snapshot,
  onChanged,
  onError,
}: {
  snapshot: Snapshot;
  onChanged: (s: Snapshot, message?: string) => void;
  onError: (message: string) => void;
}) {
  const [items, setItems] = useState<ShopItem[]>([]);
  const [busy, setBusy] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      setItems(await api.getShop());
    } catch (e) {
      onError(String(e));
    }
  }, [onError]);

  useEffect(() => {
    void load();
  }, [load, snapshot.version]);

  async function buy(item: ShopItem) {
    setBusy(item.item.id);
    try {
      const s = await api.purchaseItem(item.item.id);
      onChanged(s, `${item.item.name} delivered`);
      await load();
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function toggle(itemId: number, equipped: boolean) {
    try {
      onChanged(await api.setEquipped(itemId, equipped));
      await load();
    } catch (e) {
      onError(String(e));
    }
  }

  const equippedIds = new Set(snapshot.inventory.filter((o) => o.equipped).map((o) => o.item.id));

  return (
    <div className="page">
      <div className="shop-head">
        <h2>Shop</h2>
        <span className="coins">💰 {formatCoins(snapshot.avatar.coins)}</span>
      </div>
      <ul className="shop" data-testid="shop">
        {items.map((s) => (
          <li key={s.item.id} className="shop-item" data-owned={s.owned} data-locked={!s.unlocked}>
            <div className="shop-main">
              <span className="shop-name">{s.item.name}</span>
              <span className="shop-cost">{s.owned ? "owned" : `💰 ${s.item.cost}`}</span>
            </div>
            <p className="shop-desc">{s.item.description}</p>
            {s.owned ? (
              <button className="ghost" onClick={() => toggle(s.item.id, !equippedIds.has(s.item.id))}>
                {equippedIds.has(s.item.id) ? "Put away" : "Place in room"}
              </button>
            ) : !s.unlocked ? (
              <button disabled>Level {s.item.min_level} required</button>
            ) : (
              <button className="primary" disabled={!s.affordable || busy === s.item.id} onClick={() => buy(s)}>
                {busy === s.item.id ? "…" : s.affordable ? "Buy" : "Not enough coins"}
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
