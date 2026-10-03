--!strict
-- BN5's battle flow where it differs from BN6's (docs/design/bn5-map.md §15.3 items 7 and 13, §15.4), the arena's:
-- its Team Battle custom screen (0x08025EF2, run by the custom screen mode 0x08007F50 in place of BN6's
-- `sub_8026A28`) closes on the tick both results are in; after the reversions it opens the custom screen
-- straight away (no state 0x24); no escape check (BN6's `sub_800AAD6` is absent); the result's banner holds at
-- least 102 ticks, 65 in a special battle. And what a BN5 console shows otherwise (presentation, which the frontend
-- reads of a console's own game): its custom screen's close starts the HUD's chip window (0x080230CC), and its
-- intro fades in from black (0x080E0698 tests a byte the open bus gives).

return define.rules("bn5:flow", {
    custom_closes_with_results = true,
    sequencer_before_custom = false,
    escape_check = false,
    result_wait = { normal = 0x66, special = 65 },
    chip_window_at_close = true,
    intro_from_black = true,
})
