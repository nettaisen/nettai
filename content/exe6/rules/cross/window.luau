--!strict
-- EXE6's Cross window on the custom screen (docs/design/rules-in-luau.md
-- §4.4, S6): the Crosses offered (`sub_8029EF8`) and the window's Cross tab
-- (`sub_8026840`) as the screen opens; UP opening the window (`sub_8028B74`);
-- its opening (`sub_8027834`), the window and its keys (`sub_802794A`,
-- `sub_8028A78`), its closing (`sub_802790C`) and a Cross chosen
-- (`sub_8027A58`, `sub_8027AAE`, `sub_8027ADE`) as the system's windows; B
-- taking the Cross back (`sub_8029032`); the form it puts the navi in at
-- the turn's start (`sub_802937A`); and the round's record of the Crosses
-- used (`dword_20349A0`). Its state is the cross system's, which the
-- frontend draws the window from: its four windows are a form list's (their
-- `view`), which shows these fields by their names.

local kind = require("../forms/kind")
local chip_gate_battle = require("../../lib/chip_gate_battle")

local window = {}

-- The cross system's setup (rules/cross/init.luau declares it): the
-- player's game, the Crosses their save owns by Cross number, and nettai's
-- Cross list (up to five forms of either game, in the window's order;
-- docs/engine/custom-screen.md §4.1), none given: the version's own.
export type Setup = {
    version: "falzar" | "gregar",
    crosses: { boolean },
    cross_list: { FormDef? },
}

-- The setup's Cross list, if it gives one (its first entry set).
local function cross_list(): { FormDef? }?
    local setup = system.setup() :: Setup
    return if setup.cross_list[1] ~= nil then setup.cross_list else nil
end

-- Event flag 0x163, a navi code received (`sub_800B144` sends its level
-- only then): no Beast Out button, and the Cross window then needs the
-- navi to be MegaMan (instead of the chip gate battle).
local function sealed(side: number): boolean
    return battle.navi_level(side) ~= nil
end

-- The cross system's state fields the window keeps (rules/cross/init.luau
-- declares them).
export type State = {
    -- `dword_20349A0`: the Crosses used this round, by their place among
    -- the player's Crosses (0-4: the version's Cross number, or the place
    -- in the setup's Cross list); the round's first screen forgets them.
    crosses_used: { boolean },
    -- The screen's window (`+0x50`): the places of the Crosses offered,
    -- how many, which entry is chosen (one at most), the entry under the
    -- window's cursor, and the place of the Cross chosen.
    offered: { number },
    offered_count: number,
    marked: { boolean },
    window_cursor: number,
    cross_chosen: boolean,
    chosen: number,
}

local CROSSES = 5
-- The screen's OK slot.
local OK_SLOT = 10
-- The white fade of a Cross's choice (`sub_8027A58`), and the tick it is
-- white and the Cross is put on (`sub_8027AAE`: 16 ticks, then 8 steps of
-- the fade, then one more).
local FADE_SPEED = 0x20
local PUT_ON_TICK = 25

-- The side's navi changes form (MegaMan).
local function megaman(side: number): boolean
    return battle.navi(side).navi.forms ~= nil
end

-- `Unlocks::cross_at`: the player's Cross in place `place`: the setup's
-- Cross list's entry (of either game), else the version's Cross with that
-- number. None when the content has no such Cross.
local function cross_at(side: number, place: number): FormDef?
    local list = cross_list()
    if list ~= nil then
        return list[place + 1]
    end
    local forms = battle.navi(side).navi.forms
    if forms == nil then
        return nil
    end
    local set = if (system.setup() :: Setup).version == "gregar" then forms.gregar else forms.falzar
    if set == nil then
        return nil
    end
    return set.crosses[place + 1]
end

-- `Unlocks::owns_cross`: the save owns the Cross in place `place`, or the
-- setup's list names one there.
local function owns_cross(side: number, place: number): boolean
    local list = cross_list()
    if list ~= nil then
        return list[place + 1] ~= nil
    end
    return (system.setup() :: Setup).crosses[place + 1]
end

-- `sub_8029F70` (battle modes 0, 0xA and 0xB): not in a random battle; with
-- the link navi seal (event flag 0x163) only MegaMan, else not with
-- each side's own gauge (the chip gate battle).
local function crosses_allowed(side: number): boolean
    if custom.player(side).random_battle then
        return false
    end
    if sealed(side) then
        return megaman(side)
    end
    return not chip_gate_battle()
end

-- `sub_8029EC8`: how many Crosses the navi owns and hasn't used this round.
local function crosses_left(side: number): number
    local s = system.state() :: State
    local n = 0
    for place = 0, CROSSES - 1 do
        if owns_cross(side, place) and not s.crosses_used[place + 1] then
            n += 1
        end
    end
    return n
end

-- What a setup's Cross list (nettai's extension) offers of its entries:
-- Crosses only, and in a Beast form only the Crosses whose Beast it is
-- (that game's: their forms in Beast Out are that Beast's). Everything
-- else offers.
local function listed_cross_fits(side: number, form: FormDef): boolean
    if cross_list() == nil then
        return true
    end
    local current = battle.navi(side).form
    return form.kind == "cross" and not (kind.is_beast(current.kind) and form.version ~= current.version)
end

-- The form a Cross chosen (by its place) puts the navi in at the turn's
-- start (`sub_802937A`): the Cross, or in a Beast form its form in Beast
-- Out.
local function result_form(side: number, place: number): FormDef?
    local form = cross_at(side, place)
    if form ~= nil and kind.beast(side) then
        return form.beast
    end
    return form
end

-- `sub_802A088`: the face of the Cross chosen (by its place), its Beast
-- form's while the navi is in neither its base form nor a Cross.
local function cross_face(side: number, place: number): FormDef?
    local form = cross_at(side, place)
    if form == nil then
        return nil
    end
    local current = battle.navi(side).form
    if current.base or current.kind == "cross" then
        return form
    end
    return form.beast
end

-- The screen opens: the round's first forgets the Crosses used; the window
-- offers the Crosses owned, not used this round, and not the one the navi
-- starts battles in (`sub_8029EF8`), unless the navi is worn out; and the
-- window has the Cross tab while MegaMan has a Cross left this round.
function window.open(side: number)
    local s = system.state() :: State
    if battle.turn() == 1 then
        for place = 1, CROSSES do
            s.crosses_used[place] = false
        end
    end
    for i = 1, CROSSES do
        s.offered[i] = 0
        s.marked[i] = false
    end
    s.offered_count = 0
    s.window_cursor = 0
    s.cross_chosen = false
    s.chosen = 0
    local allowed = crosses_allowed(side) and custom.player(side).emotion ~= "worn_out"
    if allowed then
        local starting = battle.navi(side).starting_form
        for place = 0, CROSSES - 1 do
            local form = cross_at(side, place)
            if
                form ~= nil
                and owns_cross(side, place)
                and not s.crosses_used[place + 1]
                and form ~= starting
                and listed_cross_fits(side, form)
            then
                s.offered_count += 1
                s.offered[s.offered_count] = place
            end
        end
    end
    custom.set_cross_tab(side, megaman(side) and allowed and crosses_left(side) ~= 0)
end

-- OK: the Cross chosen is used this round.
function window.confirmed(side: number)
    local s = system.state() :: State
    if s.cross_chosen then
        s.crosses_used[s.chosen + 1] = true
    end
end

-- Choosing chips: UP opens the window when MegaMan has a Cross on offer,
-- no other pick holds the turn's form (Beast Out's), and the cursor is on
-- the top row or OK.
function window.keys(side: number): boolean
    local s = system.state() :: State
    if not custom.repeated(side, "up") or not megaman(side) or custom.form_taken(side) or s.offered_count == 0 then
        return false
    end
    local cursor = custom.cursor(side)
    if cursor ~= OK_SLOT and cursor > 4 then
        return false
    end
    custom.open_window(side, "cross_opening")
    return true
end

-- B with nothing picked takes the Cross chosen back.
function window.take_back(side: number): boolean
    local s = system.state() :: State
    if not s.cross_chosen then
        return false
    end
    s.marked[s.window_cursor + 1] = false
    s.cross_chosen = false
    s.chosen = 0
    custom.set_form(side, nil)
    custom.set_face(side, nil)
    custom.play(side, "cancel")
    return true
end

-- `sub_802794A`'s drawing: the window's sprites and the Cross window's
-- cursor (the frontend draws the Crosses' names from the state); the frame
-- counts on.
local function draw(side: number)
    custom.draw_window(side)
    custom.draw_cross_cursor(side)
    custom.set_frame(side, custom.frame(side) + 1)
end

-- `sub_8027834`: the window's map changes every 3 ticks (the frontend's);
-- on the 12th the Cross window is up, and its first tick runs at once.
window.opening = {
    view = "form_list_opening",
    update = function(side: number): boolean
        local tick = custom.window_tick(side)
        if tick == 1 then
            custom.play(side, "cross_window_open")
        end
        if tick >= 12 then
            custom.open_window(side, "cross_window", 1)
            custom.set_frame(side, 0)
            draw(side)
        else
            custom.set_frame(side, tick)
            custom.draw_window(side)
        end
        return true
    end,
} :: WindowSpec

-- `sub_8028A78`: the window's keys. Until a Cross is chosen, UP and DOWN
-- move the cursor (wrapping) and A chooses the Cross under it (a chosen one
-- refused); then B closes the window, START closes it with the screen's
-- cursor on OK, and R describes the Cross under the cursor (its own, of
-- whichever game it is).
local function keys(side: number)
    local s = system.state() :: State
    if not s.cross_chosen then
        local n = s.offered_count
        local up, down = custom.repeated(side, "up"), custom.repeated(side, "down")
        if up or down then
            if up then
                s.window_cursor = if s.window_cursor == 0 then n - 1 else s.window_cursor - 1
            elseif s.window_cursor + 1 >= n then
                s.window_cursor = 0
            else
                s.window_cursor += 1
            end
            if n > 1 then
                custom.play(side, "cursor")
            end
            return
        end
        if custom.pressed(side, "a") then
            local i = s.window_cursor + 1
            if s.marked[i] then
                custom.play(side, "refused")
                return
            end
            custom.play(side, "pick")
            s.marked[i] = true
            s.cross_chosen = true
            s.chosen = s.offered[i]
            custom.set_form(side, result_form(side, s.chosen))
            custom.open_window(side, "cross_chosen")
            -- A chosen Cross grays out Beast Out (its button's state).
            custom.refresh_buttons(side)
            return
        end
    end
    if custom.pressed(side, "b") then
        custom.open_window(side, "cross_closing")
    elseif custom.pressed(side, "start") then
        custom.set_cursor(side, OK_SLOT)
        custom.open_window(side, "cross_closing")
        custom.play(side, "cursor")
    elseif custom.pressed(side, "r") then
        custom.describe(side, cross_at(side, s.offered[s.window_cursor + 1]))
        custom.play(side, "description")
    end
end

-- `sub_802794A`: its first tick reads no keys; every tick draws, after the
-- keys.
window.window = {
    view = "form_list",
    update = function(side: number): boolean
        if custom.window_tick(side) == 1 then
            custom.set_frame(side, 0)
        else
            keys(side)
        end
        draw(side)
        return true
    end,
} :: WindowSpec

-- `sub_802790C`: after 5 ticks the window is the chips' again
-- (`sub_80279FC`, `sub_80279C8`, `sub_8028476`).
window.closing = {
    view = "form_list_closing",
    update = function(side: number): boolean
        local tick = custom.window_tick(side)
        if tick == 1 then
            custom.play(side, "cross_window_close")
        end
        custom.set_frame(side, tick)
        if tick >= 6 then
            custom.draw_window(side)
            custom.show_chip_window(side)
        end
        custom.draw_emblem(side, 0)
        custom.draw_regular(side)
        return tick < 6
    end,
} :: WindowSpec

-- `sub_8027A58`: 16 ticks, then the screen fades to white and back; when it
-- is white the Cross is put on (`sub_8027AAE`: the face, the window, the
-- sound), and when it is clear again the chips are chosen (`sub_8027ADE`).
window.chosen = {
    view = "form_chosen",
    update = function(side: number): boolean
        local s = system.state() :: State
        local tick = custom.window_tick(side)
        if tick == 1 then
            custom.set_frame(side, 0)
        elseif tick <= 16 then
            custom.set_frame(side, custom.frame(side) + 1)
        elseif tick == 17 then
            custom.set_frame(side, 0)
            custom.fade(side, "end_to_white", FADE_SPEED)
        elseif tick == PUT_ON_TICK then
            custom.fade(side, "intro_from_white", FADE_SPEED)
            if s.cross_chosen then
                custom.set_face(side, cross_face(side, s.chosen))
            end
            custom.draw_window(side)
            custom.show_chip_window(side)
            custom.play(side, "cross_chosen")
        elseif tick == 34 then
            custom.draw_window(side)
            custom.show_chip_window(side)
        end
        custom.draw_emblem(side, 0)
        custom.draw_regular(side)
        return tick < 34
    end,
} :: WindowSpec

return window
