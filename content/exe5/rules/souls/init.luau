--!strict
-- EXE5's Soul Unison (docs/design/exe5-map.md §5, §15.8): MegaMan unites with a soul for some turns. The custom
-- screen's soul button (./custom: it gives up the last chip picked for the soul of its family, a soul a form with a
-- `soul` naming its `family`) asks for it in the turn's transform record; at the turn's start the sequencer runs the
-- soul's change (./change). The part keeps the side's soul turns (AIData +0x0F) and whether it is Chaos Unison,
-- counts the turns down as each later custom screen is asked for and asks for the revert when they run out.
--
-- A soul is a form, named by its id, and what is one soul's is in its folder (navis/megaman/forms/<soul>): its
-- weapons, its navi's image at the change (its `image`) and what it adds to the custom screen (its `custom`: its
-- buttons, windows and part of the screen's routines, which ./custom gathers from the forms). What is here is the
-- part's: the button and the soul's choice, the turns, the change, the revert, Chaos Unison's failure.

local change = require("@self/change")
local custom_souls = require("@self/custom")
local revert = require("@self/revert")

local souls = {}

export type State = {
    -- AIData +0x0F: the soul's turns left (0xFF: none to count).
    turns: number,
    -- Chaos Unison.
    chaos: boolean,
}

-- What the rules list of Soul Unison's: its actions (the change, the revert) and what it adds to the custom screen
-- (./custom: the soul button and window, and each soul's own additions, gathered from the souls' forms), with the
-- state fields each soul's addition keeps (each soul's own, by its form: navis/megaman/forms/<soul>).
souls.change_action = change.action
souls.revert_action = revert.action
souls.custom = custom_souls

-- 0x0801246C, 0x0801243A: in a soul whose turns ran out, MegaMan asks to revert.
function souls.turn_check(side: number)
    local navi = battle.player(side) :: Object
    local s = rules.state() :: State
    if battle.navi(side).form.soul ~= nil and s.turns == 0 then
        navi:set_request("revert_form", true)
    end
end

-- 0x0801248C, which EXE5 calls as a custom screen is asked for mid-fight (0x08007774, EXE6's `sub_8008452`'s place,
-- where EXE6 counts its Beast Out down: not at the turn's start, so a soul's first turn shows all its turns): MegaMan's
-- soul turns count down (not below 0, and not 0xFF), unless the side's navi isn't MegaMan (NaviStats +0x29).
function souls.custom_requested(side: number)
    local s = rules.state() :: State
    if battle.navi(side).navi.id == "megaman" and s.turns ~= 0xFF and s.turns ~= 0 then
        s.turns -= 1
    end
end

return souls
