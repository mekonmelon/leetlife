//! Susuwatari silhouette, with fuzzy edges and oversized eyes at Retina scale.
//! Template alpha stays readable in light and dark macOS menu bars.
pub const WIDTH: u32 = 64;
pub const HEIGHT: u32 = 44;
pub fn pixels(frame: u8, sleepy: bool) -> Vec<u8> {
    let mut rgba = vec![0; (WIDTH * HEIGHT * 4) as usize];
    let breath = if frame % 4 < 2 { 0.0 } else { 0.7 };
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut alpha = 0;
            for sy in [0.25, 0.75] {
                for sx in [0.25, 0.75] {
                    let px=x as f32+sx;
                    let py=y as f32+sy+breath;
                    let dx=px-28.0;
                    let dy=py-25.0;
                    let angle=dy.atan2(dx);
                    let radius=15.0+(angle*29.0).sin().abs()*2.0+(angle*43.0).sin()*0.6;
                    let body=dx*dx+dy*dy<radius*radius;
                    let oval=|cx:f32,cy:f32,rx:f32,ry:f32|((px-cx)/rx).powi(2)+((py-cy)/ry).powi(2)<1.0;
                    let blink=sleepy||frame==3;
                    let eyes=if blink {oval(21.5,25.0,5.4,1.4)||oval(34.5,24.0,5.4,1.4)} else {oval(21.5,24.0,5.5,6.0)||oval(34.5,23.0,5.5,6.0)};
                    let pupils=!blink&&(oval(22.0,24.0,1.7,2.0)||oval(34.0,23.0,1.7,2.0));
                    let z=sleepy&&frame%4>=2&&px>49.0&&px<57.0&&((py-9.0).abs()<0.8||(py-17.0).abs()<0.8||(px+py-66.0).abs()<0.8&&py>9.0&&py<17.0);
                    if body&&(!eyes||pupils)||z {alpha+=1;}
                }
            }
            rgba[((y*WIDTH+x)*4+3) as usize]=(alpha*255/4) as u8;
        }
    }
    rgba
}
