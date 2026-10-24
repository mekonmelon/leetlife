# LeetLife artwork

Active character: **Susuwatari**, the familiar fuzzy black soot sprite with
large white eyes. The house and menu bar drawings are in
`src/avatar/Avatar.tsx` and `src-tauri/src/app/tray_art.rs`. The original leaf
creature and invented animal companions have been replaced.

## House reference

Production asset: `public/art/reference-house.png`.

Created with the built-in imagegen tool from the user's attached house image.
The exterior shows the tall, crooked timber-and-plaster house with its sweeping
red tile roofs, balconies, chimney, flower boxes, front door, lawn, and blue sky.
Room buttons and clickable doors enter the existing saved decorating spaces.

Final prompt:

> Image 1 is the exact house the user chose. Use Image 1 as the house and preserve its architectural identity faithfully: the very tall whimsical stacked Tudor house, all of its crooked dark timber beams, creamy pale yellow plaster, multiple steep sweeping weathered terracotta red tile roofs, dormer windows, tall narrow pale stone chimney, little balconies, bottom front doorway and path, flower boxes, laundry, grassy foreground and clear blue sky. Do not design a different house, do not simplify into a generic cottage, do not turn it into vector art. Preserve the reference painting's proportions, silhouette, details, palette, painterly brushwork and sunny atmosphere. Produce a clean production game background showing the entire same house and a little surrounding lawn, at a near-square portrait composition, with the full house from chimney to front path visible. No new characters, no text, no UI, no frame, no watermark.

## Existing Ghibli companions

Production asset: `public/art/ghibli-companions.png` (1536 × 1024, transparent).
Created with the built-in imagegen tool using the user's character collage.
This is reference-based character artwork, not an official film asset pack.

Final prompt:

> Create exactly six existing Studio Ghibli character cutouts in a strict uniform 3-column by 2-row grid on a transparent background. Row 1: Haku in his recognizable white serpentine dragon form from Spirited Away, turquoise mane and long whiskers; Jiji from Kiki's Delivery Service, the slender black cat with big white eyes and tall ears; blue Chū Totoro with his leaf bundle and tiny white Chibi Totoro beside him. Row 2: Boh as a plump grey mouse with Yubaba's little black bird perched on his head; Ootori-sama, the fluffy yellow bath spirit with an orange beak and folded white towel; Aogaeru, the green frog attendant in his pale blue robe. Preserve their existing film designs. No original species or invented reinterpretations. Soft traditional 2D cel-animation linework and painted shading. Fit each full character inside its own equal-sized cell without overlap. No text, borders, interface or watermark.

The characters are cropped to their painted bounds for display by
`src/house/GhibliCompanion.tsx` without modifying
the raster image. The old inventory IDs remain stable, so previously collected
companions and decorations retain their ownership and room placement.

The interior's distant garden scenery remains in `public/art/woodland-dusk.png`.
All artwork ships in the app. No runtime image service or subscription is used.
