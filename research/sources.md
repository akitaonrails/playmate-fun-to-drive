# Sources

Retrieved 2026-10-15. Images downloaded into `images/` for private reference.

## Identification & history

- Toy Tales — "Turnin' Turbo Dashboard from TOMY (1983)"
  https://toytales.ca/turnin-turbo-dashboard-from-tomy-1983/
  (controls, gameplay/scoring rules, crash behavior; 4 reference images incl. schematic)
- Nostalgia Nerd — "Playmates Fun to Drive Corvette Dashboard"
  https://nostalgianerd.com/playmates-dashboard/
  (the Playmates toy: pop-up lights, centered screen, 2 s loop, no crash, hi/lo gear, 1985 Hong Kong)
- Tectoy — Wikipédia (pt) https://pt.wikipedia.org/wiki/Tectoy
  (company founded 1987 for high-tech toys; "Turbo Driver" in product list)
- "TURBO VÍDEO DRIVER da TECTOY" video https://www.youtube.com/watch?v=8odf_Pa2HUk
  (1991, hybrid mechanical/electronic, volante + câmbio + ignition)
- Vintage Turbo Video Driver video https://www.youtube.com/watch?v=vJDaPfqtNDU
- Auction listings for the Tectoy unit (lights/sound/track movement, 15×25×25 cm):
  https://www.coisaantigaleiloes.com.br/peca.asp?ID=11468561
  https://www.nossacolecao.com.br/peca.asp?id=17650800
  https://lista.mercadolivre.com.br/turbo-video-driver-tectoy
- eBay listings for "Fun-To-Drive Corvette Dashboard by Playmates 1985" (photos)
- Reddit r/nostalgia threads on the Playmates toy (photos, memories)

## Mechanism / teardowns

- Circuitbeard — "Tomy Turnin' Turbo Dashboard Outrun Arcade" (2017)
  https://circuitbeard.co.uk/2017/08/28/tomy-turnin-turbo-dashboard-outrun-arcade/
  (steering rotary→linear actuator; shifter sweeping gear + self-center; ignition key wiring;
  screen/drum cavity photos; his OutRun-on-a-toy build is the direct ancestor of THIS project's step 5)
- lutfionboard — "Fixing a Tomy Turnin' Turbo Dashboard clone" (2026-02-16)
  https://lutfionboard.com/post/2026-02-16-my-post
  (clean mechanism-front/side, drum/screen, headlights-on photos)
- Ashens teardown video (via Bitmob) https://bitmob.com/articles/the-tomy-turnin-turbo-dashboard-autopsy.html
- CarShowz summary (lamp + rotating drum description) https://carshowz.com/turbo-dashboard-outrun-arcade/
- Tomy teardown/repair/play video https://www.youtube.com/watch?v=heU8jOWXtRQ

## Related builds (inspiration)

- Indie Retro News — Playmates 1985 Fun-to-Drive converted to Android/MAME
  https://www.indieretronews.com/2016/10/playmates-1985-fun-to-drive-converted.html
- Vice/Co.Create coverage of the Circuitbeard mini OutRun cabinet
  https://www.vice.com/en/article/this-tiny-fully-functioning-out-run-arcade-cabinet-is-too-adorbs/

## Tooling research (astra on codex)

- Local `codex` CLI 0.162.1, logged in via ChatGPT subscription
  (`codex login status`).
- Model cache contains **`gpt-6-astra`** (and `gpt-5.6-sol` as the user's
  default) — `tools/astra/` pins `-m gpt-6-astra` by default.
