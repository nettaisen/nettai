--!strict
-- BN5's Soul Unison (docs/design/bn5-map.md §5): MegaMan unites with a soul for some turns. The custom screen's soul
-- button (the framework's, BN5's screen: it gives up the last chip picked for the soul of its family, a soul a form
-- of `kind = "soul"` naming its `soul.family`) asks for it in the turn's transform record; at the turn's start the
-- sequencer runs the soul's change (./change). The system keeps the side's soul turns (AIData +0x0F) and whether it
-- is Chaos Unison, counts the turns down as each later custom screen is asked for and asks for the revert when they
-- run out.

local change = require("./change")
local revert = require("./revert")

local souls = {}

export type State = {
    -- AIData +0x0F: the soul's turns left (0xFF: none to count).
    turns: number,
    -- Chaos Unison.
    chaos: boolean,
}

-- The soul change, as the pause handler runs it.
souls.action = define.action {
    id = "bn5:souls/change",
    state = { timer = "u16" },
    update = change.update,
}

-- The revert out of a soul (./revert), as the pause handler runs it.
souls.revert = define.action {
    id = "bn5:souls/revert",
    state = { timer = "u16" },
    update = revert.update,
}

souls.system = define.system {
    id = "bn5:souls",
    state = { turns = "u8", chaos = "bool" },
    -- The souls the player has (a match's side's `souls`, which nettai-match writes by this field's name; a
    -- recording's, its version's: bn5-compat), which the custom screen's soul button may offer: the engine reads them
    -- as the save's soul flags (0x08024BF0's, by each soul's number). Any game's souls, either version's.
    setup = { souls = "form[16]" },
    actions = { souls.action, souls.revert },
    hooks = {
        -- 0x0801246C, 0x0801243A: in a soul whose turns ran out, MegaMan asks to revert.
        turn_check = function(side: number)
            local navi = battle.player(side) :: Object
            local s = system.state() :: State
            if battle.navi(side).form.soul ~= nil and s.turns == 0 then
                navi:set_request("revert_form", true)
            end
        end,
        -- 0x0801248C, which BN5 calls as a custom screen is asked for mid-fight (0x08007774, BN6's `sub_8008452`'s
        -- place, where BN6 counts its Beast Out down: not at the turn's start, so a soul's first turn shows all its
        -- turns): MegaMan's soul turns count down (not below 0, and not 0xFF), unless the side's navi isn't
        -- MegaMan (NaviStats +0x29).
        custom_requested = function(side: number)
            local s = system.state() :: State
            if battle.navi(side).navi.id == "bn5:megaman" and s.turns ~= 0xFF and s.turns ~= 0 then
                s.turns -= 1
            end
        end,
    },
}

return souls
