--!strict
-- EXE5's Soul Unison (docs/design/exe5-map.md §5, §15.8): MegaMan unites with a soul for some turns. The custom
-- screen's soul button (./custom: it gives up the last chip picked for the soul of its family, a soul a form with a
-- `soul` naming its `family`) asks for it in the turn's transform record; at the turn's start the sequencer runs the
-- soul's change (./change). The system keeps the side's soul turns (AIData +0x0F) and whether it is Chaos Unison,
-- counts the turns down as each later custom screen is asked for and asks for the revert when they run out.

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

-- The soul change, as the pause handler runs it.
souls.action = define.action {
    id = "souls/change",
    state = { timer = "u16" },
    update = change.update,
}

-- The revert out of a soul (./revert), as the pause handler runs it.
souls.revert = define.action {
    id = "souls/revert",
    state = { timer = "u16" },
    update = revert.update,
}

-- The system's state: its own fields, the soul button's, and what each soul's addition to the custom screen keeps
-- (./custom's `states`).
local state: { [string]: string | { string } } = {
    turns = "u8",
    chaos = "bool",
    -- The custom screen's (./custom): the round's souls given, the button's offer, the soul's choice's step.
    souls_used = "u32",
    offer = "form",
    offer_number = "u8",
    offer_chaos = "bool",
    unite_step = "u8",
    unite_count = "u8",
}
for _, fields in custom_souls.states do
    for name, kind in fields do
        state[name] = kind
    end
end

souls.system = define.system {
    id = "souls",
    state = state,
    -- What the player's save brings: the souls it has (a match's side's `souls`, which nettai-match writes by this
    -- field's name; a recording's, its version's: exe5-compat), which the custom screen's soul button may offer, as the
    -- save's soul flags (0x08024BF0's, by each soul's number; any souls of the game, either version's); Soul Unison
    -- (event flag 0: the soul button) and Chaos Unison (event flag 0x236: a dark chip's soul). A finished save has
    -- both, as a setup that says nothing.
    setup = { souls = "form[16]", soul_unison = "bool", chaos_unison = "bool" },
    setup_defaults = { soul_unison = true, chaos_unison = true },
    actions = { souls.action, souls.revert },
    -- Soul Unison on the custom screen, and what each soul adds to it (./custom).
    buttons = custom_souls.buttons,
    windows = custom_souls.windows,
    custom = {
        hand_size = custom_souls.hand_size,
        open = custom_souls.open,
        deal = custom_souls.deal,
        confirmed = custom_souls.confirmed,
    },
    hooks = {
        -- 0x08011DDC: the turn's requests read (ColonelSoul's arm chip).
        turn_opened = custom_souls.turn_opened,
        -- 0x0801246C, 0x0801243A: in a soul whose turns ran out, MegaMan asks to revert.
        turn_check = function(side: number)
            local navi = battle.player(side) :: Object
            local s = system.state() :: State
            if battle.navi(side).form.soul ~= nil and s.turns == 0 then
                navi:set_request("revert_form", true)
            end
        end,
        -- 0x0801248C, which EXE5 calls as a custom screen is asked for mid-fight (0x08007774, EXE6's `sub_8008452`'s
        -- place, where EXE6 counts its Beast Out down: not at the turn's start, so a soul's first turn shows all its
        -- turns): MegaMan's soul turns count down (not below 0, and not 0xFF), unless the side's navi isn't
        -- MegaMan (NaviStats +0x29).
        custom_requested = function(side: number)
            local s = system.state() :: State
            if battle.navi(side).navi.id == "megaman" and s.turns ~= 0xFF and s.turns ~= 0 then
                s.turns -= 1
            end
        end,
    },
}

return souls
