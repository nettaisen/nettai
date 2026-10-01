# Object kinds seen in PvP fixtures

From the golden traces (machgun + soundmod). T1 = actors (navis, summons), T3 = attacks, T4 = effects.
Handler = jumptable entry (T1 0x08003C9C, T3 0x08003EC4, T4 0x080042C8).

| kind | handler | frames alive (both traces) |
|---|---|---|
| T1 0x00 | 0x080b81ec | 119138 |
| T1 0x05 | 0x080b8cd8 | 9798 |
| T1 0x06 | 0x080b8ea0 | lab only (BlastMan: the pack's `objects/blast-man`, chips.md §3.6.14) |
| T1 0x07 | 0x080b9078 | lab only (HeatMan: the pack's `objects/heat-man`, chips.md §3.6.15) |
| T1 0x08 | 0x080b92b8 | lab only (ElecMan: the pack's `objects/elec-man`, chips.md §3.6.16) |
| T1 0x09 | 0x080b94bc | 545 (SpoutMan: the pack's `objects/spout-man`, chips.md §3.6.11) |
| T1 0x0a | 0x080b97c0 | lab only (TomahawkMan: the pack's `objects/tomahawk-man`, chips.md §3.6.12) |
| T1 0x0c | 0x080b9c14 | lab only (TenguMan: the pack's `objects/tengu-man`, chips.md §3.6.13) |
| T1 0x0d | 0x080b9f44 | lab only (SlashMan: the pack's `objects/slash-man`, chips.md §3.6.18) |
| T1 0x0f | 0x080ba708 | 438 |
| T1 0x10 | 0x080baa8c | 481 (ElmntMan: the pack's `objects/elmnt-man`, chips.md §3.6.7) |
| T1 0x15 | 0x080bb608 | 513 (EraseMan: the pack's `chips/eraseman/navi`, chips.md §3.6.7) |
| T1 0x16 | 0x080bb914 | lab only (ChargeMan: the pack's `objects/charge-man`, chips.md §3.6.17) |
| T1 0x1b | 0x080bc650 | 424 (Cross navi image: `kinds::cross_merge`, objects-and-player.md §12.10) |
| T1 0x2d | 0x080c0e04 | 136 (navi warp: `kinds::navi_warp`, chips.md §3.6.7) |
| T1 0x50 | 0x080c3ce8 | 454 |
| T1 0x55 | 0x080c40d8 | 592 (SpoutMan's idle overlay: `kinds::idle_overlay`, put on by the navi hooks in `kinds::player::form`) |
| T1 0x56 | 0x080c4348 | 72155 (body overlay: `kinds::body_overlay`, objects-and-player.md §12.10) |
| T1 0x57 | 0x080c4530 | 1148 |
| T1 0x5d | 0x080c4828 | 300 |
| T3 0x00 | 0x080c4e58 | 3189 (projectile: the pack's `objects/projectile`, objects-and-player.md §B8) |
| T3 0x03 | 0x080c52b0 | 975 |
| T3 0x07 | 0x080c5a34 | 15623 (volcano eruption: `kinds::eruption`, field-collision-damage.md) |
| T3 0x09 | 0x080c5ddc | 1452 (panel strike: the pack's `objects/panel-strike`, chips.md §3.6.33) |
| T3 0x0b | 0x080c60a8 | 38 (flying shot: the pack's `objects/flying-shot`, objects-and-player.md §B8) |
| T3 0x0f | 0x080c6414 | 1107 (grab shot: the pack's `lib/grab/shot`, chips.md §3.6.8) |
| T3 0x12 | 0x080c6946 | 600 (the Vulcans' and Spreaders' bullet: shot-chips.md §4.1, not ported) |
| T3 0x17 | 0x080c6dcc | 180 (SpoutMan's geyser: the pack's `objects/spout-geyser`, chips.md §3.6.11) |
| T3 0x21 | 0x080c8388 | lab only (BlastMan's fire blast: the pack's `objects/blast-fire`, chips.md §3.6.14) |
| T3 0x22 | 0x080c853c | 16 (SpoutMan's ball: the pack's `objects/spout-ball`, chips.md §3.6.11) |
| T3 0x23 | 0x080c86d8 | lab only (its splash: the pack's `objects/spout-splash`, chips.md §3.6.11) |
| T3 0x26 | 0x080c8c74 | lab only (HeatMan's flame: the pack's `chips/heatman/flame`, chips.md §3.6.15) |
| T3 0x49 | 0x080cd2ec | 87 (WindRack's gust: the pack's `objects/gust`, standard-chips.md) |
| T3 0x59 | 0x080cf954 | 7372 (rock: the content's `objects/rock`, a definition with its variants and debris, field-objects.md §3) |
| T3 0x5b | 0x080cfcf8 | 96 |
| T3 0x62 | 0x080d07cc | lab only (SlashMan's sword wave: the pack's `objects/slash-wave`, chips.md §3.6.18) |
| T3 0x64 | 0x080d0d7c | lab only (ElecMan's thunderbolt: the pack's `objects/elec-thunder`, chips.md §3.6.16) |
| T3 0x6e | 0x080d2290 | lab only (a stage's boulder: the content's `objects/boulder`, field-objects.md §3.1) |
| T3 0x74 | 0x080d30d0 | 350 (RskyHny's bee: the pack's `chips/rskyhny/bee`, chips.md §3.7) |
| T3 0x7d | 0x080d4c84 | lab only (the Guardian statue, a stage's too: the content's `chips/guardian/statue`, dimming-chip-effects.md §5) |
| T3 0x82 | 0x080d5740 | 48 (DolThdr's doll: the pack's `chips/dolthdr/doll`, standard-chips.md) |
| T3 0x8b | 0x080d6924 | 225 (DolThdr's thunder column: the pack's `objects/thunder-column`, standard-chips.md) |
| T3 0x8d | 0x080d6bd4 | 38 (meteor: the pack's `objects/meteor`, chips.md §3.6.7) |
| T3 0x8e | 0x080d6d80 | 96 (ElmntMan's ice: the pack's `objects/elmnt-ice`, chips.md §3.6.7) |
| T3 0x94 | 0x080d7acc | 217 |
| T3 0xac | 0x080dae94 | lab only (ChargeMan's train car: the pack's `objects/charge-car`, chips.md §3.6.17) |
| T3 0xaf | 0x080db570 | 2197 |
| T3 0xb0 | 0x080db6a4 | 1852 (DustCross junk ball: the content's `navis/00-megaman/forms/dustcross/junk_ball`, objects-and-player.md §B6) |
| T3 0xb4 | 0x080dbcec | 512 (MetrKnuk's fist: the pack's `chips/metrknuk/fist`, dimming-chip-effects.md §18) |
| T3 0xb8 | 0x080dc3f8 | lab only (ElmntMan's bolt: the pack's `objects/elmnt-bolt`, chips.md §3.6.7) |
| T3 0xb9 | 0x080dc4fc | 96 (ElmntMan's vine: the pack's `objects/elmnt-vine`, chips.md §3.6.7) |
| T3 0xc1 | 0x080dd34c | 296 |
| T3 0xc2 | 0x080dd764 | 967 |
| T3 0xc3 | 0x080dd940 | 732 (EraseMan's slash: the pack's `chips/eraseman/beam`, chips.md §3.6.7) |
| T3 0xc8 | 0x080de13c | 2774 (a dragon's body segment: the pack's `lib/dragons/body`, chips.md §3.8) |
| T3 0xc9 | 0x080de404 | 579 (a dragon's head: the pack's `lib/dragons/head`, chips.md §3.8) |
| T3 0xcf | 0x080df328 | 4170 |
| T4 0x00 | 0x080e0548 | 3910 |
| T4 0x02 | 0x080e0638 | 250 |
| T4 0x03 | 0x080e0710 | 1876 (AreaGrab dimming controller: the pack's `lib/grab/controller`, chips.md §3.6.8) |
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
| T4 0x2d | 0x080e37f4 | 316 (SpoutMan's pillar: the pack's `objects/spout-pillar`, chips.md §3.6.11) |
| T4 0x2e | 0x080e39a0 | 810 (SpoutMan's geyser marks: the pack's `objects/spout-mark`, chips.md §3.6.11) |
| T4 0x2f | 0x080e3ab8 | 471 |
| T4 0x3b | 0x080e4910 | 494 |
| T4 0x48 | 0x080e5c2c | 847 (GunDelSol's sun beam: the pack's `chips/gundels/beam`, chips.md §4) |
| T4 0x5a | 0x080e70c8 | 184 |
| T4 0x5d | 0x080e74d4 | 762 (Invisibl dimming controller: the pack's `objects/invisible`, chips.md §3.6) |
| T4 0x62 | 0x080e78bc | 260 (EraseMan's aim mark: the pack's `chips/eraseman/mark`, chips.md §3.6.7) |
| T4 0x6b | 0x080e807c | 54 |
| T4 0x76 | 0x080e8b00 | 595 (MetrKnuk's dimming controller: the pack's `chips/metrknuk/controller`, dimming-chip-effects.md §18) |
| T4 0x80 | 0x080e9460 | 1192 |
| T4 0x84 | 0x080e9570 | 761 |
| T4 0x87 | 0x080e97f0 | 20 (absorbed obstacle: the content's `objects/absorbed-obstacle`, a definition whose looks are its obstacles' records, field-objects.md §5) |
| T4 0x89 | 0x080e9af0 | 118 |
