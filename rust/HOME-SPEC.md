# Home 1:1 spec (extracted from upstream web + Electron values)

Light theme tokens:
- background #fcfcfc; card/popover #ffffff
- foreground #262626; foreground/95 labels; foreground/89 row idle text
- muted-foreground ≈ #676767; muted-foreground/58 rail glyph rest tone
- secondary = black/3% → sidebar hover/selected fill ≈ #f7f7f7
- muted black/4%; border black/5%; input black/6%; ring #a3a3a3
- font: -apple-system/system-ui; heading font-display "Cal Sans" fallback
- text-ui 13px; ui-sm 12px; ui-lg ~14px; heading 30px normal tracking -0.015em

Icon rail (left, 51px = 3.25rem-1px):
- column pt/pb 10px, gap 6px, buttons 36x36 rounded-8
- top: sidebar-toggle (two-panel glyph), home(active=bg black/3% + ink icon), inbox, [tasks cap], list, expand, toggle, clock, 4-dot-grid, "…" more
- divider h-px w-20px between groups; bottom pinned: "?" help, person account
- idle glyph muted/58, hover bg secondary + ink glyph

Top bar: transparent, "+" new-thread at content left, right icons (inbox pill + pane/restore).

Sidebar (~ 165-230px, translucent surface card 38%):
- header: "Synara ⌄" bold ~17px + right: search icon, small icon button
- "✎ New thread" text row (13px, pencil glyph, idle)
- "Projects" section label (13px muted); "No projects yet" centered muted 13px
- "Chats >" collapsible section row
- hairline divider on its right edge

Content home (empty landing):
- AsciiWaveBackground: 10px glyphs, 13px line-height, random A-Za-z0-9, ink alpha ≤0.18 inside drifting noise blotches only — sparse columns left+right of center
- Sponsor banner top-center, max-w-520px, hidden <620px height: rounded-2xl (16px) px-16 py-12, star 20px in 40px box (primary color), "Support Synara" ui medium + "Explore sponsorships and help fund development" ui muted; × 22px circle at -6px top-right (border/70, bg-background, opacity on hover)
- Center: SynaraLogo 40px + 16px gap + "What should we work on?" 30px normal fg/95
- Above composer: "Work in a project" ghost chip (folder icon 14px + 12-13px text) in tray, rounded-top-only shell flush to composer
- Composer card: squircle radius 19px, white, soft shadow 0 4px 18px -6px fg/7%; placeholder "Do anything in Synara" muted; footer row: "+" 28px circle outline, "⚠ Full access ⌄" amber-500 text 12px; right: "▶ GPT-6 Astra ⌄" + send arrow in grey circle 28px
- pane bottom padding 12-16px
