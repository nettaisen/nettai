# Object kinds seen in PvP fixtures

From the golden traces (machgun + soundmod). T1 = actors (navis, summons), T3 = attacks, T4 = effects.
Handler = jumptable entry (T1 0x08003C9C, T3 0x08003EC4, T4 0x080042C8).

| kind | handler | frames alive (both traces) |
|---|---|---|
| T1 0x00 | 0x080b81ec | 119138 |
| T1 0x05 | 0x080b8cd8 | 9798 |
| T1 0x09 | 0x080b94bc | 545 |
| T1 0x0f | 0x080ba708 | 438 |
| T1 0x10 | 0x080baa8c | 481 (ElmntMan: `kinds::elmnt_man`, chips.md §3.6.7) |
| T1 0x15 | 0x080bb608 | 513 (EraseMan: the pack's `objects/erase-man`, chips.md §3.6.7) |
| T1 0x1b | 0x080bc650 | 424 (Cross navi image: `kinds::cross_merge`, objects-and-player.md §12.10) |
| T1 0x2d | 0x080c0e04 | 136 (navi warp: `kinds::navi_warp`, chips.md §3.6.7) |
| T1 0x50 | 0x080c3ce8 | 454 |
| T1 0x55 | 0x080c40d8 | 592 |
| T1 0x56 | 0x080c4348 | 72155 (body overlay: `kinds::body_overlay`, objects-and-player.md §12.10) |
| T1 0x57 | 0x080c4530 | 1148 |
| T1 0x5d | 0x080c4828 | 300 |
| T3 0x00 | 0x080c4e58 | 3189 (projectile: the pack's `objects/projectile`, objects-and-player.md §B8) |
| T3 0x03 | 0x080c52b0 | 975 |
| T3 0x07 | 0x080c5a34 | 15623 (volcano eruption: `kinds::eruption`, field-collision-damage.md) |
| T3 0x09 | 0x080c5ddc | 1452 |
| T3 0x0b | 0x080c60a8 | 38 (flying shot: the pack's `objects/flying-shot`, objects-and-player.md §B8) |
| T3 0x0f | 0x080c6414 | 1107 (grab shot: the pack's `objects/grab-shot`, chips.md §3.6.8) |
| T3 0x12 | 0x080c6946 | 600 |
| T3 0x17 | 0x080c6dcc | 180 |
| T3 0x22 | 0x080c853c | 16 |
| T3 0x49 | 0x080cd2ec | 87 (WindRack's gust: the pack's `objects/gust`, standard-chips.md) |
| T3 0x59 | 0x080cf954 | 7372 (rock: the pack's `objects/rock`, field-objects.md) |
| T3 0x5b | 0x080cfcf8 | 96 |
| T3 0x74 | 0x080d30d0 | 350 (RskyHny's bee: the pack's `objects/honey-bee`, chips.md §3.7) |
| T3 0x82 | 0x080d5740 | 48 (DolThdr's doll: the pack's `objects/thunder-doll`, standard-chips.md) |
| T3 0x8b | 0x080d6924 | 225 (DolThdr's thunder column: the pack's `objects/thunder-column`, standard-chips.md) |
| T3 0x8d | 0x080d6bd4 | 38 (meteor: `kinds::meteor`, chips.md §3.6.7) |
| T3 0x8e | 0x080d6d80 | 96 |
| T3 0x94 | 0x080d7acc | 217 |
| T3 0xaf | 0x080db570 | 2197 |
| T3 0xb0 | 0x080db6a4 | 1852 (DustCross junk ball: the pack's `objects/dust-ball`, objects-and-player.md §B6) |
| T3 0xb4 | 0x080dbcec | 512 |
| T3 0xb9 | 0x080dc4fc | 96 |
| T3 0xc1 | 0x080dd34c | 296 |
| T3 0xc2 | 0x080dd764 | 967 |
| T3 0xc3 | 0x080dd940 | 732 (EraseMan's slash: the pack's `objects/erase-beam`, chips.md §3.6.7) |
| T3 0xc8 | 0x080de13c | 2774 (a dragon's body segment: the pack's `objects/dragon-body`, chips.md §3.8) |
| T3 0xc9 | 0x080de404 | 579 (a dragon's head: the pack's `objects/dragon-head`, chips.md §3.8) |
| T3 0xcf | 0x080df328 | 4170 |
| T4 0x00 | 0x080e0548 | 3910 |
| T4 0x02 | 0x080e0638 | 250 |
| T4 0x03 | 0x080e0710 | 1876 (AreaGrab dimming controller: the pack's `objects/area-grab`, chips.md §3.6.8) |
| T4 0x04 | 0x080e0844 | 1134 |
| T4 0x07 | 0x080e0ad4 | 5502 |
| T4 0x08 | 0x080e0df0 | 118268 |
| T4 0x0a | 0x080e10a4 | 56 |
| T4 0x0f | 0x080e1520 | 6154 |
| T4 0x10 | 0x080e17e8 | 6284 (navi chip dimming controller: `kinds::navi_chip`, chips.md §3.6.7) |
| T4 0x1c | 0x080e23a4 | 334 |
| T4 0x1f | 0x080e28a8 | 192 |
| T4 0x20 | 0x080e2ae8 | 609 |
| T4 0x28 | 0x080e32b8 | 500 |
| T4 0x2a | 0x080e34c0 | 2276 (trap chip dimming controller: the pack's `objects/trap-chip`, chips.md §3.6.9) |
| T4 0x2d | 0x080e37f4 | 316 |
| T4 0x2e | 0x080e39a0 | 810 |
| T4 0x2f | 0x080e3ab8 | 471 |
| T4 0x3b | 0x080e4910 | 494 |
| T4 0x48 | 0x080e5c2c | 847 |
| T4 0x5a | 0x080e70c8 | 184 |
| T4 0x5d | 0x080e74d4 | 762 (Invisibl dimming controller: the pack's `objects/invisible`, chips.md §3.6) |
| T4 0x62 | 0x080e78bc | 260 (EraseMan's aim mark: the pack's `objects/erase-mark`, chips.md §3.6.7) |
| T4 0x6b | 0x080e807c | 54 |
| T4 0x76 | 0x080e8b00 | 595 |
| T4 0x80 | 0x080e9460 | 1192 |
| T4 0x84 | 0x080e9570 | 761 |
| T4 0x87 | 0x080e97f0 | 20 (absorbed obstacle: `kinds::absorbed_obstacle`, field-objects.md) |
| T4 0x89 | 0x080e9af0 | 118 |
