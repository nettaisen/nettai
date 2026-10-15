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
local dark = require("@self/dark")
local megaman = require("../navis/megaman").megaman
local revert = require("@self/revert")
local turns = require("@self/turns")

local souls = {}

-- A soul's button as the rules': on the screen while the side's navi is in the soul (the screen's +0x0F), its
-- functions called with the rules' state.
local function button_of(soul: FormDef, button: SoulButton): ButtonSpec
    return {
        slot = button.slot,
        cells = button.cells,
        uses = button.uses,
        shown = function(side: number): boolean
            return battle.navi(side).form == soul
        end,
        pressed = function(side: number)
            button.pressed(side, rules.state())
        end,
    }
end

-- A soul's window as the rules'.
local function window_of(window: SoulWindow): WindowSpec
    return {
        view = window.view,
        update = function(side: number): boolean
            return window.update(side, rules.state())
        end,
    }
end

-- What the rules list of Double Soul's: its actions, which reach the rules' state (./turns): the revert, each
-- soul's change (its chip's use, ./change's maker), gathered from MegaMan's souls (his form list), and the dark
-- soul's (./dark); and its part
-- of the custom screen: the UNITE button and the soul's choice (./custom), and what each soul adds to the screen
-- (its form's `custom`: its buttons and windows by name, a button's the key of its look in a pack, and the state
-- fields it keeps). A name is one of Double Soul's: two souls can't both add a button, a window or a field of one.
local actions: { Action } = { revert.action, dark.change }
local buttons: { [string]: ButtonSpec } = { soul = custom.button }
local windows: { [string]: WindowSpec } = { double_soul = custom.window }
local screen: StateSchema = {}
local function once(names: { [string]: any }, name: string, what: string, soul: FormDef)
    if names[name] ~= nil then
        error(string.format("%s's %s `%s` is another's too: a name is one of rules/souls's", tostring(soul.id), what, name))
    end
end
local forms: { [string]: any } = megaman.forms or {}
for _, set in forms do
    local list: { FormDef } = set.form_list or {}
    for _, form in list do
        local change: Action = assert(form.change, "a soul names its change")
        table.insert(actions, change)
        local part: SoulCustom? = form.custom
        if part ~= nil then
            for name, button in part.buttons or {} :: { [string]: SoulButton } do
                once(buttons, name, "button", form)
                buttons[name] = button_of(form, button)
            end
            for name, window in part.windows or {} :: { [string]: SoulWindow } do
                once(windows, name, "window", form)
                windows[name] = window_of(window)
            end
            for name, kind in part.state or {} :: StateSchema do
                once(screen, name, "screen field", form)
                screen[name] = kind
            end
        end
    end
end
souls.actions = actions
souls.custom = custom
souls.dark = dark
souls.buttons = buttons
souls.windows = windows

-- What a side keeps of Double Soul through a round: the soul's turns (./turns: AIData +0x18 counted up toward +0x19,
-- which the emotion window counts down from: the offer view's roles), the souls given this round (0x0203BF88: a soul
-- given is one the save has, of its six; a build has as many as setup's `souls` holds, twelve, of both versions),
-- and the custom screen's (./custom): the UNITE button's offer (the screen's +0x0D; once given, the soul given) and
-- the soul's choice's step and count; and what each soul's screen keeps.
souls.state = {
    soul_turns = schema.role("form_offer.turns", "u8"),
    soul_turn_limit = schema.role("form_offer.turn_limit", "u8"),
    souls_used = schema.list("form", 12),
    offer = schema.role("form_offer.form", "form"),
    unite_step = schema.role("offer_flight.step", "u8"),
    unite_count = schema.role("offer_flight.count", "u8"),
    soul_hand_sent = "bool",
} :: StateSchema
for name, kind in screen do
    assert(souls.state[name] == nil, `a soul's screen field \`{name}\` is one of Double Soul's own`)
    souls.state[name] = kind
end
-- And the dark soul's (./dark).
for name, kind in dark.state do
    souls.state[name] = kind
end

-- 0x0800B658, each tick of MegaMan's intake but a dimmed one (`battle_isTimeStop`): held by his change (the frozen
-- flag, 0x10000) he asks for nothing; with a soul's chip next in his hand (its family special, 15: 0x0800B924's chip
-- less 0x160, the soul OK gave, which the engine's change targets) he asks for the change; in a soul whose turns are
-- over (AIData +0x18 at its +0x19) for the revert (+0x44's 0x40), and in the dark soul (NaviStats +0x24 15) once its
-- ticks are (./dark, 0x0800B6A4); else for nothing (+0x10 0xFF). Then, as the status routine looks for a change to
-- start (0x0800B8B0), the dark soul asked for is the one, whatever this found (./dark); and in the dark soul no hit
-- reaches him this tick (0x0800C43C, the intake's next call).
function souls.navi_intake(side: number, navi: Object)
    if not battle.dimmed() then
        if navi:status("frozen") then
            navi:set_request("form_change", false)
        else
            local next = navi:next_chip()
            if next ~= nil and next.family == "special" then
                navi:set_request("form_change", true)
            else
                navi:set_request("form_change", false)
                if dark.united(side) then
                    dark.countdown(navi)
                elseif battle.navi(side).form.soul ~= nil and turns.over() then
                    navi:set_request("revert_form", true)
                end
            end
            dark.asked(side, navi)
        end
    end
    dark.navi_intake(side, navi)
end

-- 0x08012FB6, each tick of a player's intake (0x08013858; the navis' own, 0x0801393E, has none) after the panel's
-- standing effects (0x08012FF2), not while dimmed: in WoodSoul (NaviStats +0x24 12) on grass (the panel of its
-- collision), a point of HP back (`object_addHP`) when the battle's second count of fight ticks comes round
-- (BattleState +0x16: every other tick).
local HEALS_ON_GRASS = "woodsoul"
function souls.grass_heal(side: number, navi: Object)
    if battle.dimmed() or battle.player(side) ~= navi then
        return
    end
    local c = navi.collision
    local p = field.panel(c.panel_x, c.panel_y)
    if p == nil or p.kind ~= "grass" or battle.navi(side).form.id ~= HEALS_ON_GRASS or battle.cycle(2) ~= 0 then
        return
    end
    navi.hp = math.min(navi.hp + 1, navi.max_hp)
end

-- 0x0800BA20, as either player asks for a custom screen: the soul's turns count one.
souls.custom_requested = turns.custom_requested

return souls
