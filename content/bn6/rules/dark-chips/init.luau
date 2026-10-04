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
--
-- The bug frags are this system's too (`dword_203F7E0`): the player brings
-- the save's (the setup's `bug_frags`, which the init exchange shares), and
-- the battle spends them from its state. Chips that spend one (the navi
-- boosts' charged shots) ask BN6's API (rules/api.luau: `bn6.bug_frags`,
-- `bn6.spend_bug_frags`), which reaches this state.

export type State = { bug_frags: number }

-- The HP bug's highest level (NaviStats+0x18).
local MAX_HP_BUG = 7

return define.system {
    id = "dark-chips",
    setup = { bug_frags = "u32" },
    state = { bug_frags = "u32" },
    extends = {
        chip = {
            dark_substitute = "chip",
            hp_bug = "u8",
        },
    },
    hooks = {
        -- The round starts with the bug frags the player brought.
        round_setup = function(_side: number)
            local s = system.state() :: State
            s.bug_frags = (system.setup() :: State).bug_frags
        end,
        -- `sub_8010D58`: a bug frag spent, or the substitute.
        chip_substitute = function(_side: number, _navi: Object, chip: Chip): Chip?
            local substitute = chip.dark_substitute
            if substitute == nil then
                return nil
            end
            local s = system.state() :: State
            if s.bug_frags >= 1 then
                -- sub_800F4B2 (the local player's save loses one too).
                s.bug_frags -= 1
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
