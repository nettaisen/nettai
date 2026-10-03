--!strict
-- BN5's emotions (docs/design/bn5-map.md §15.10). The emotion itself is the framework's by BN5's rules (the status
-- section's `emotions`: 0x08012740's order, 0x080127D6's setter, the anger tick); the starting mood and the palette
-- go by the light/dark value (rules/light-dark). What BN5 decides here:
--
-- - **Full Synchro on a counter** (0x08016FDC, BN6's `sub_801A200`): the counterer's side, in no soul (NaviStats
--   +0x2C 0), to mood 0xFF through BN5's setter, which leaves a mood of 0 (a dark MegaMan's) as it is.

return define.system {
    id = "bn5:emotion",
    hooks = {
        countered = function(side: number, _victim: Object)
            if battle.navi(side).form.kind == "base" then
                battle.set_mood(side, 0xFF)
            end
        end,
    },
}
