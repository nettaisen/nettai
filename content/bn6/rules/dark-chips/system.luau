--!strict
-- BN6's dark chips (docs/engine/chips.md): a dark chip, one with a
-- `dark_substitute`, costs its user a bug frag as its use is prepared
-- (`sub_8010D58`, `sub_800F4B2`); with none left the user gets the
-- substitute instead (`off_8010D84`: Sword, Thunder, Recov10, Invisibl,
-- Atk+10 for the five dark chips), with its own record, damage and bonus.
-- Using a dark chip worsens its user's HP bug by its `hp_bug`, to 7 at most
-- (`sub_800B79A`: DrkSword +2, DarkThnd +1, DrkRecov to 7, DarkPlus +4). The
-- two fields are this system's extension of chips (docs/design/rules-in-luau.md
-- §7.5): the engine checks them and reads neither.

-- The HP bug's highest level (NaviStats+0x18).
local MAX_HP_BUG = 7

return define.system {
    id = "dark-chips",
    extends = {
        chip = {
            dark_substitute = "chip",
            hp_bug = "u8",
        },
    },
    hooks = {
        -- `sub_8010D58`: a bug frag spent, or the substitute.
        chip_substitute = function(side: number, _navi: Object, chip: Chip): Chip?
            local substitute = chip.dark_substitute
            if substitute == nil then
                return nil
            end
            if battle.bug_frags(side) >= 1 then
                -- sub_800F4B2 (the local player's save loses one too).
                battle.spend_bug_frags(side, 1)
                return nil
            end
            return substitute
        end,
        -- `sub_800B79A`: the HP bug worsened by the chip a use was prepared
        -- with (on each path: a use, the rush's chain, the counter cut-in's;
        -- not a form's weapon run instead, which prepares none).
        chip_prepared = function(side: number, _navi: Object, chip: Chip)
            local cost = chip.hp_bug
            if cost == nil or cost == 0 then
                return
            end
            local stats = battle.navi(side)
            stats.hp_drain = math.min(stats.hp_drain + cost, MAX_HP_BUG)
        end,
    },
}
