--!strict
-- BN5's Soul Unison (docs/design/bn5-map.md §5): MegaMan unites with a soul for some turns. The custom screen's soul
-- button (the framework's, BN5's screen: it gives up the last chip picked for the soul of its family, a soul a form
-- of `kind = "soul"` naming its `soul.family`) asks for it in the turn's transform record; at the turn's start the
-- sequencer runs the soul's change (./change). The system keeps the side's soul turns (AIData +0x0F) and whether it
-- is Chaos Unison, counts the turns down and asks for the revert when they run out.

local change = require("./change")

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

souls.system = define.system {
    id = "bn5:souls",
    state = { turns = "u8", chaos = "bool" },
    actions = { souls.action },
    hooks = {
        -- 0x0801246C, 0x0801243A: in a soul whose turns ran out, MegaMan asks to revert.
        turn_check = function(side: number)
            local navi = battle.player(side) :: Object
            local s = system.state() :: State
            if battle.navi(side).form.kind == "soul" and s.turns == 0 then
                navi:set_request("revert_form", true)
            end
        end,
        -- 0x0801248C: a turn starts: MegaMan's soul turns count down (not below 0, and not 0xFF).
        turn_started = function(side: number)
            local s = system.state() :: State
            if battle.navi(side).navi.id == "bn5:megaman" and s.turns ~= 0xFF and s.turns ~= 0 then
                s.turns -= 1
            end
        end,
    },
}

return souls
