--!strict
-- EXE4's Double Soul (docs/design/exe4-map.md §6), as EXE5's rules/souls is its Soul Unison: with Double Soul (event
-- flag 0x14) the custom screen's UNITE button gives up the last chip picked for the soul of its family, and OK puts
-- the soul's chip first in the hand (./custom); with it next in the hand the intake asks for the change, which
-- MegaMan's status routine starts as the fight runs (./change, the soul's own: action 0x0D); the soul lasts 3 custom
-- screens (./turns), then the intake asks for the revert, which his idle starts (./revert, action 0x11).
--
-- A soul is a form, named by its id (navis/megaman/forms/<soul>); what a soul does in battle, its own status routine
-- (0x0800E0A0 by soul), is the soul's (docs/design/exe4-map.md §18 item 74).

local custom = require("@self/custom")
local megaman = require("../navis/megaman").megaman
local revert = require("@self/revert")
local turns = require("@self/turns")

local souls = {}

-- What the rules list of Double Soul's: its actions, which reach the rules' state (./turns): the revert, and each
-- soul's change (its chip's use, ./change's maker), gathered from MegaMan's souls (his form list); and its part
-- of the custom screen.
local actions: { Action } = { revert.action }
local forms: { [string]: any } = megaman.forms or {}
for _, set in forms do
    local list: { FormDef } = set.form_list or {}
    for _, form in list do
        local change: Action = assert(form.change, "a soul names its change")
        table.insert(actions, change)
    end
end
souls.actions = actions
souls.custom = custom

-- What a side keeps of Double Soul through a round: the soul's turns (./turns: AIData +0x18 counted up toward +0x19,
-- which the emotion window counts down from: the offer view's roles), the souls given this round (0x0203BF88: a soul
-- given is one the save has, of its six), and the custom screen's (./custom): the UNITE button's offer (the screen's
-- +0x0D; once given, the soul given) and the soul's choice's step and count.
souls.state = {
    soul_turns = schema.role("form_offer.turns", "u8"),
    soul_turn_limit = schema.role("form_offer.turn_limit", "u8"),
    souls_used = schema.list("form", 6),
    offer = schema.role("form_offer.form", "form"),
    unite_step = schema.role("offer_flight.step", "u8"),
    unite_count = schema.role("offer_flight.count", "u8"),
} :: StateSchema

-- 0x0800B658, each tick of MegaMan's intake but a dimmed one (`battle_isTimeStop`): held by his change (the frozen
-- flag, 0x10000) he asks for nothing; with a soul's chip next in his hand (its family special, 15: 0x0800B924's chip
-- less 0x160, the soul OK gave, which the engine's change targets) he asks for the change; in a soul whose turns are
-- over (AIData +0x18 at its +0x19) for the revert (+0x44's 0x40); else for nothing (+0x10 0xFF). (The dark soul's
-- countdown, NaviStats +0x24 15, is the story's: no netbattle's navi is in it.)
function souls.navi_intake(side: number, navi: Object)
    if battle.dimmed() then
        return
    end
    if navi:status("frozen") then
        navi:set_request("form_change", false)
        return
    end
    local next = navi:next_chip()
    if next ~= nil and next.family == "special" then
        navi:set_request("form_change", true)
        return
    end
    navi:set_request("form_change", false)
    if battle.navi(side).form.soul ~= nil and turns.over() then
        navi:set_request("revert_form", true)
    end
end

-- 0x0800BA20, as either player asks for a custom screen: the soul's turns count one.
souls.custom_requested = turns.custom_requested

return souls
