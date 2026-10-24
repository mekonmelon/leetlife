# Current game artwork

The house and backyard are original, code-drawn pixel scenes. The reference
screenshots informed the isometric cutaway perspective and game presentation;
their artwork, buildings and layouts are not copied into the game.

- `src/game/pixels.ts`: stepped pixel shapes, isometric floor tiles, timber
  walls, furniture, curtains, plants, lighting, garden plots and flowers.
- `src/house/life.ts`: floor paths and twelve connected stair tiles; continuous
  walking and activity states for tea, reading, studying, sweeping and sleeping.
- `src/house/House.tsx`: animation and tile interaction; one house view and one
  unlockable backyard, with no room background images.
- `src/game/PixelPreview.tsx`: shop previews drawn with the same pixel renderer.
- `src/game/game.css`: game HUD, action dock and menus.

Susuwatari is the main character. Familiar Ghibli companions are represented by
small code-drawn pixel sprites. No original animal species were added. The
existing inventory IDs and room placement coordinates are retained for saves.

The former painted assets and their imagegen prompts are archived in
`assets/previous-art/`. They are not part of `public/` and do not ship in the app.
The current scenes require no image generation service, download or subscription.
