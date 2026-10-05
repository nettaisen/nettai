--!strict
-- EXE5's Soul Unison (docs/design/exe5-map.md §5, §15.8): MegaMan unites with a soul for some turns. The custom
-- screen's soul button (./custom: it gives up the last chip picked for the soul of its family, a soul a form with a
-- `soul` naming its `family`) asks for it in the turn's transform record; at the turn's start the sequencer runs the
-- soul's change (./change). The system keeps the side's soul turns (AIData +0x0F) and whether it is Chaos Unison,
-- counts the turns down as each later custom screen is asked for and asks for the revert when they run out.
--
-- A soul is a form, named by its id, and what is one soul's is in its folder (navis/megaman/forms/<soul>): its
-- weapons, its navi's image at the change (its `image`) and what it adds to the custom screen (its `custom`: its
-- buttons, windows and part of the screen's routines, which ./custom gathers from the forms). What is here is the
-- system's: the button and the soul's choice, the turns, the change, the revert, Chaos Unison's failure.

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

-- The system's state: its own fields, the soul button's, and what each soul's addition to the custom screen keeps
-- (./custom's `states`).
local state: { [string]: string | { string } } = {
    turns = "u8",
    chaos = "bool",
    -- The turn's arm chip (the transform record's +6; none: its 0xFFFF).
    arm_chip = "chip",
    -- The custom screen's (./custom): the round's souls given, the button's offer, the soul's choice's step.
    souls_used = "u32",
    offer = "form",
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
    -- save's soul flags (0x08024BF0's, one a soul; any souls of the game, either version's); Soul Unison
    -- (event flag 0: the soul button) and Chaos Unison (event flag 0x236: a dark chip's soul). A finished save has
    -- both, as a setup that says nothing.
    setup = { souls = "form[16]", soul_unison = "bool", chaos_unison = "bool" },
    setup_defaults = { soul_unison = true, chaos_unison = true },
    actions = { change.action, revert.action },
    -- What a soul's form says for the rules that ask (a soul is named by its form's id; the original tests its
    -- number, NaviStats +0x2C, in each place; the pack's types.d.luau types them): its navi's image at the change
    -- (./image, a `SoulImage`) and what it adds to the custom screen (./custom, a `SoulCustom`); its sword's arm, the
    -- blade's animation (lib/swords, 0x080EC038); its charged sword warping behind the enemy whatever the charge
    -- (lib/stepsword, 0x08012538); VarSwrd's wait for its sequence while A is up (chips/varswrd, 0x080ED570); the
    -- cameras' shake as MegaMan emerges in it (./change, 0x0801212C).
    extends = {
        form = {
            image = "record:soul-image",
            custom = "record:soul-custom",
            blade_anim = "u8",
            steps_behind = "bool",
            var_sword_waits = "bool",
            emerge_shake = "bool",
        },
    },
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
