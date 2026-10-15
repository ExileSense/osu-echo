# Site themes

A theme is a folder here: `themes/<id>/` with
  - `index.html`  (required – the page shell; keep the element ids that profile.js fills in)
  - `theme.json`  (optional – `{"name": "...", "description": "..."}` shown in the picker)
  - any css/img files, referenced as `/site/static/themes/<id>/...`

Rules
  - Put `class="theme-<id>"` on `<body>`; scope all CSS with `body.theme-<id>` and gate any
    theme-only JS in profile.js on that class (see `NEXT` for the 2017 theme).
  - New folders are listed automatically in the footer "Site theme" picker (`GET /site/themes`).
    Embedded themes need a rebuild; a folder placed in `website/static/themes/` next to the exe
    shows up with no rebuild.
  - The choice is a `site_theme` cookie, so switching is instant (page reload, no server restart).
    `SITE_THEME=<id>` in `.env` is only the default for browsers without that cookie.
  - Bump the `?v=` on your css link when you change it (static files are cached for 24h).
