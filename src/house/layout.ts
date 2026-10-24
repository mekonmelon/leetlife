import type { ItemDef, Placement, RoomId } from "../api/types";
export const ROOMS: { id: RoomId; name: string; subtitle: string; icon: string }[] = [
  { id: "hearth", name: "The hearth", subtitle: "A warm cup & a wandering thought", icon: "tea" },
  { id: "studio", name: "The atelier", subtitle: "Where little ideas take root", icon: "star" },
  { id: "loft", name: "The moon loft", subtitle: "For dreaming up impossible things", icon: "moon" },
];
export const WALLPAPERS = [
  { asset: "wallpapers/linen", name: "Old linen", color: "#e2d6b9", ink: "#b5aa89", pattern: "linen" },
  { asset: "wallpapers/fern", name: "Fern & fable", color: "#b9c3a5", ink: "#7d946f", pattern: "fern" },
  { asset: "wallpapers/starlight", name: "Midnight wishes", color: "#949cb5", ink: "#e5d9aa", pattern: "stars" },
  { asset: "wallpapers/rose", name: "Wild rose", color: "#dcc0b9", ink: "#a3807e", pattern: "rose" },
];
const ANCHORS: Record<string, Placement> = {
  "furniture/plant": {room:"hearth",x:268,y:183}, "furniture/desk": {room:"studio",x:238,y:181},
  "furniture/bed": {room:"loft",x:91,y:184}, "furniture/couch": {room:"hearth",x:91,y:188},
  "furniture/poster": {room:"studio",x:76,y:91}, "furniture/lamp": {room:"hearth",x:250,y:157},
  "furniture/bookshelf": {room:"studio",x:78,y:164}, "furniture/pc": {room:"studio",x:258,y:184},
  "pets/cat": {room:"loft",x:215,y:200}, "pets/dog": {room:"hearth",x:111,y:202},
  "furniture/rug": {room:"hearth",x:164,y:205}, "furniture/window": {room:"loft",x:234,y:102},
  "pets/dragon": {room:"hearth",x:228,y:202}, "furniture/whale-painting": {room:"loft",x:80,y:95},
  "furniture/mushroom-lamp": {room:"hearth",x:240,y:177}, "furniture/tea-table": {room:"hearth",x:171,y:186},
  "pets/boh": {room:"hearth",x:115,y:202}, "pets/ootori": {room:"loft",x:210,y:201}, "pets/aogaeru": {room:"studio",x:240,y:200},
  "furniture/cloud-cushion": {room:"loft",x:198,y:202},
};
export const defaultPlacement = (item: ItemDef): Placement => ANCHORS[item.asset_path] ?? {room:"hearth",x:160,y:190};
export const canPlace = (item: ItemDef) => !["wallpaper", "room", "outfit"].includes(item.category);
export const isWallArt = (asset: string) => /poster|painting|window/.test(asset);
export function clampPlacement(placement: Placement, asset: string): Placement {
  return {...placement, x: Math.max(30, Math.min(290, placement.x)), y: Math.max(isWallArt(asset) ? 45 : 155, Math.min(isWallArt(asset) ? 135 : 210, placement.y))};
}
export function randomVisit(previous?: RoomId) {
  const choices = ROOMS.filter(room => room.id !== previous);
  return { room: choices[Math.floor(Math.random()*choices.length)].id, x: 85+Math.random()*140, y: 190+Math.random()*21 };
}
