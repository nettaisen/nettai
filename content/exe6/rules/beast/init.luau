--!strict
-- EXE6's Beast Out (docs/design/rules-in-luau.md §3.1): a navi's turns in
-- Beast Out and when they run out; the rush (rush.luau, the game's wrapper:
-- the role `actions.wrapper`); Beast Over's berserk (berserk.luau, the
-- controller of its `controlled` forms) and its exhaustion; and the Cross
-- special (cross-special.luau, DarkInvs' takeover), whose controller shares
-- the berserk's state, as the original's share their 0x10 bytes. Their
-- state is the rules' (rules/init.luau). The turn-start sequencer is the framework's (EXE5 has it
-- too); it asks each side's rules at its check and when the turn starts.
-- Beast Out on the custom screen (its button, its animations and the
-- BeastOut chip's) is custom.luau.

local beast_claw = require("../navis/megaman/weapons/beast_claw")
local berserk = require("@self/berserk")
local cross_special = require("@self/cross_special")
local kind = require("./forms/kind")
local custom_beast = require("@self/custom")
local rush = require("@self/rush")
local slash_charge = require("../navis/megaman/forms/slashcross/charge")

-- The side's form runs its charged sword inside the rush (SlashCross
-- Beast's `charged_sword_rush`).
local function sword_rush(side: number): boolean
    return battle.navi(side).form.charged_sword_rush == true
end

export type State = {
    -- The rush's (rush.luau).
    rush_phase: number,
    rush_started: boolean,
    rush_home_x: number,
    rush_home_y: number,
    rush_chain: boolean,
    rush_chain_block: number,
    rush_countdown: number,
    -- The controllers' (berserk.luau, cross-special.luau).
    controller_step: number,
    controller_moves: number,
    controller_volley: number,
    controller_started: boolean,
    -- AIData+0x0F: the turn-start check (`sub_80159C6`) runs only while
    -- this is 0, and then sets it to 2. Closing the custom screen sets it to
    -- 1 (`sub_8009338`); a mid-battle custom-screen request counts it down
    -- (`sub_8015A16`, MegaMan only; 0xFF is left alone). Intent uncertain:
    -- in netbattles the close always leaves 1 before the next check.
    check_delay: number,
    -- The custom screen's (custom.luau).
    beast_out_used: boolean,
}

-- The side's navi changes form (MegaMan): where the original asks whether
-- the navi is MegaMan.
local function changes_form(side: number): boolean
    return battle.navi(side).navi.forms ~= nil
end

local beast = {}

-- What the rules list of Beast Out's: the rush and the Cross special, their
-- actions; Beast Out on the custom screen (custom.luau: its button, its
-- windows and its part of the screen's routines).
beast.rush_action = rush.action
beast.cross_special_action = cross_special.action
beast.custom = custom_beast

-- `sub_800FB54`: what runs inside the rush. In a Beast form (or
-- from the Cross special), a chip with the lock-on flag (its
-- `beast`, unless `rush = false`); of the forms' charged weapons,
-- the Beast forms' claw and SlashCross Beast's charged sword (the
-- form's `charged_sword_rush`).
function beast.chip_used(side: number, navi: Object, chip: Chip, weapon: Weapon?)
    if weapon ~= nil then
        local action = navi:navi_action()
        if action == beast_claw.action or (action == slash_charge.action and sword_rush(side)) then
            navi.wrapped = 1
        end
        return
    end
    if navi.special_source ~= 0 or kind.beast(side) then
        local rush_of = chip.beast
        navi.wrapped = if rush_of ~= nil and rush_of.rush ~= false then 1 else 0
    end
end

-- `sub_802D322`: Beast Over's berserk decides a controlled navi's
-- idle.
beast.controller = berserk.control
-- `sub_802E4E4`, `sub_802D4C6`: the Cross special.
beast.takeover_requested = cross_special.requested
beast.takeover = cross_special.takeover
-- `sub_80158CC`: outside battle mode 1, reverting spends a Beast Out
-- and exhausts a Beast Over.
function beast.form_reverted(side: number, navi: Object)
    if battle.mode() == 1 then
        return
    end
    local form_kind = battle.navi(side).form.kind
    if form_kind == "beast" or form_kind == "cross_beast" then
        navi.tired = true
    elseif form_kind == "beast_over" then
        -- `sub_8014466`: exhausted, then the mood 0 that exhaustion
        -- itself blocks.
        navi.exhausted = true
        battle.set_mood(side, 0)
    end
end

-- `sub_8009338`: the custom screen closed.
function beast.custom_closed(side: number)
    local s = rules.state() :: State
    s.check_delay = 1
end

-- `sub_8015A16`: a mid-battle custom screen was asked for.
function beast.custom_requested(side: number)
    if not changes_form(side) then
        return
    end
    local s = rules.state() :: State
    if s.check_delay ~= 0 and s.check_delay ~= 0xFF then
        s.check_delay -= 1
    end
end

-- `sub_80159C6` + `sub_8015994`, once per turn start: the check
-- runs only while the delay is 0 (and sets it to 2), and a Beast Out
-- whose counter ran out asks to revert (request 0x40, handled while
-- paused). In battle mode 1 a Beast form always reverts.
function beast.turn_check(side: number)
    local navi = battle.player(side) :: Object
    local stats = battle.navi(side)
    local revert: boolean
    if battle.mode() == 1 then
        revert = kind.beast(side)
    else
        local s = rules.state() :: State
        if s.check_delay ~= 0 then
            return
        end
        s.check_delay = 2
        if stats.beast_out_counter ~= 0 then
            return
        end
        navi.tired = true
        revert = kind.beast(side)
    end
    if revert then
        navi:set_request("revert_form", true)
    end
end

-- `sub_8015A38`: a turn in Beast Out uses up one of MegaMan's turns,
-- unless he started the battle in Beast Out.
function beast.turn_started(side: number)
    local stats = battle.navi(side)
    local started_beast = stats.starting_form.kind == "beast"
    if changes_form(side) and not started_beast and kind.beast(side) and stats.beast_out_counter ~= 0 then
        stats.beast_out_counter -= 1
    end
end

return beast
