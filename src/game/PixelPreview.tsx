import { useEffect, useRef } from "react";
import { drawItem, project } from "./pixels";
export function PixelPreview({asset}:{asset:string}) {
  const canvas=useRef<HTMLCanvasElement>(null);
  useEffect(()=>{if(typeof CanvasRenderingContext2D==="undefined")return;const c=canvas.current?.getContext("2d");if(!c)return;c.clearRect(0,0,100,78);c.imageSmoothingEnabled=false;const p=project(0,0);c.save();c.translate(50-p[0],59-p[1]);drawItem(c,asset,0,0);c.restore();},[asset]);
  return <canvas className="pixel-preview" ref={canvas} width={100} height={78} aria-hidden="true"/>;
}
