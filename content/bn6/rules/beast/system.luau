--!strict
-- BN6's Beast Out (docs/design/rules-in-luau.md §3.1): a navi's turns in
-- Beast Out and when they run out. The turn-start sequencer is the
-- framework's (BN5 has it too); it asks each side's systems at its check
-- and when the turn starts.
--
-- (Still the framework's until later slices: the Beast forms' change
-- sequences, the rush, Beast Over, the Beast Out button.)

export type State = {
    -- AIData+0x0F: the turn-start check (`sub_80159C6`) runs only while
    -- this is 0, and then sets it to 2. Closing the custom screen sets it to
    -- 1 (`sub_8009338`); a mid-battle custom-screen request counts it down
    -- (`sub_8015A16`, MegaMan only; 0xFF is left alone). Intent uncertain:
    -- in netbattles the close always leaves 1 before the next check.
    check_delay: number,
}

-- The side's navi changes form (MegaMan): where the original asks whether
-- the navi is MegaMan.
local function changes_form(side: number): boolean
    return battle.navi(side).navi.forms ~= nil
end

return define.system {
    id = "bn6:beast",
    state = { check_delay = "u8" },
    hooks = {
        -- `sub_8009338`: the custom screen closed.
        custom_closed = function(side: number)
            local s = system.state() :: State
            s.check_delay = 1
        end,
        -- `sub_8015A16`: a mid-battle custom screen was asked for.
        custom_requested = function(side: number)
            if not changes_form(side) then
                return
            end
            local s = system.state() :: State
            if s.check_delay ~= 0 and s.check_delay ~= 0xFF then
                s.check_delay -= 1
            end
        end,
        -- `sub_80159C6` + `sub_8015994`, once per turn start: the check
        -- runs only while the delay is 0 (and sets it to 2), and a Beast Out
        -- whose counter ran out asks to revert (request 0x40, handled while
        -- paused). In battle mode 1 a Beast form always reverts.
        turn_check = function(side: number)
            local navi = battle.player(side) :: Object
            local stats = battle.navi(side)
            local revert: boolean
            if battle.mode() == 1 then
                revert = stats.beast
            else
                local s = system.state() :: State
                if s.check_delay ~= 0 then
                    return
                end
                s.check_delay = 2
                if stats.beast_out_counter ~= 0 then
                    return
                end
                navi.beast_out_spent = true
                revert = stats.beast
            end
            if revert then
                navi:set_request("revert_form", true)
            end
        end,
        -- `sub_8015A38`: a turn in Beast Out uses up one of MegaMan's turns,
        -- unless he started the battle in Beast Out.
        turn_started = function(side: number)
            local stats = battle.navi(side)
            local started_beast = stats.starting_form.kind == "beast"
            if changes_form(side) and not started_beast and stats.beast and stats.beast_out_counter ~= 0 then
                stats.beast_out_counter -= 1
            end
        end,
    },
}
