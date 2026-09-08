# Intent layer — localisation keys (LANDSCAPE_2200 §2196)

Strings are not the source of truth. Every player-facing phrase is an **intent**
with a stable id; languages bind later. Do not scatter literals that belong here.

| Intent id | English (en) | Used by |
|---|---|---|
| `hud.habitat.title` | KEPLER DRUM | HUD habitat panel |
| `hud.you.title` | COLONIST | HUD you panel |
| `hud.ground.title` | GROUND | HUD ground panel |
| `hud.weather.title` | WEATHER | HUD weather panel |
| `menu.resume` | Resume | Pause menu |
| `menu.quit` | Quit | Pause menu |
| `menu.first_run` | First run — set accessibility & content, then Resume. | First-run gate |
| `caption.excavating` | excavating | Audio captions |
| `caption.placed` | placed | Audio captions |
| `caption.splash` | splash | Audio captions |
| `toast.photo_on` | photo mode | Photo mode |
| `toast.photo_off` | HUD back | Photo mode |
| `away.days` | while you were away: {days} habitat-days passed | Resume blurb |
| `content.archive` | Archive memories enabled | Content settings |
| `content.romance` | Romance intensity | Content settings |

When adding a readout or toast, add a row here first, then reference the id.
