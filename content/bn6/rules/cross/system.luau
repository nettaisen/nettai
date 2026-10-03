--!strict
-- BN6's Crosses on the custom screen (docs/design/rules-in-luau.md §3.1,
-- S6): the Cross window (window.luau), ChargeCross's extra chips and the
-- hand size they go into (`sub_802A49C`, `sub_802A40C`), and DustCross's
-- scrap button.

local window = require("./window")

type State = {
    -- `dword_20349A0`: consecutive screens opened in ChargeCross (or its
    -- Beast form), up to 3: each deals one more chip. The round's first
    -- screen forgets it.
    charge_cross_screens: number,
}

-- Whether the side's form has `trait`.
local function form_has(side: number, trait: string): boolean
    for _, t in battle.navi(side).form.traits or {} do
        if t == trait then
            return true
        end
    end
    return false
end

-- `sub_802A49C` and `sub_802A40C`: ChargeCross (a form with `extra_chips`)
-- deals one more chip per screen spent in it, up to three; the custom
-- level and those chips, 8 at most (the rest kept as extra); NumbrOpn's 10
-- (not in DustCross, whose scrap button takes its place), the extra then
-- ChargeCross's; and the hand-shrink bug takes the extra first, then the
-- hand, to 2 at least.
local function hand_size(side: number): number
    local s = system.state() :: State
    local stats = battle.navi(side)
    local turn = battle.turn()
    if turn == 1 then
        s.charge_cross_screens = 0
    end
    if stats.navi.forms ~= nil and form_has(side, "extra_chips") then
        s.charge_cross_screens = math.min(s.charge_cross_screens + 1, 3)
    else
        s.charge_cross_screens = 0
    end
    local ccs = s.charge_cross_screens
    local extra = 0
    local n = stats.custom_level + ccs
    if n > 8 then
        extra = n - 8
        n = 8
    end
    if not form_has(side, "scrap_button") and stats.number_open then
        n = 10
        extra = ccs
    end
    local bug = stats.hand_shrink_turn
    if bug ~= 0 and turn >= bug then
        extra -= turn - bug + 1
        if extra < 0 then
            n = math.max(n + extra, 2)
        end
    end
    return n
end

-- DustCross's scrap button (`sub_8027F10`), two cells wide on slots 8 and
-- 9, usable once: with a chip picked last (`sub_8028F84`), A scraps the
-- picks (`sub_8028E04`).
local scrap: ButtonSpec = {
    slot = 8,
    cells = 2,
    uses = 1,
    right = 11,
    left = 7,
    shown = function(side: number): boolean
        return battle.navi(side).navi.forms ~= nil and form_has(side, "scrap_button")
    end,
    state = function(side: number): "selectable" | "unavailable"
        return if custom.last_pick_is_chip(side) then "selectable" else "unavailable"
    end,
    pressed = function(side: number)
        if custom.cursor_state(side) == "selectable" then
            custom.sacrifice(side)
        else
            custom.refuse(side)
        end
    end,
}

return define.system {
    id = "bn6:cross",
    state = {
        charge_cross_screens = "u8",
        -- The Cross window's (window.luau).
        crosses_used = "bool[5]",
        offered = "u8[5]",
        offered_count = "u8",
        marked = "bool[5]",
        window_cursor = "u8",
        cross_chosen = "bool",
        chosen = "u8",
    },
    buttons = { scrap = scrap },
    windows = {
        cross_opening = window.opening,
        cross_window = window.window,
        cross_closing = window.closing,
        cross_chosen = window.chosen,
    },
    custom = {
        hand_size = hand_size,
        open = window.open,
        confirmed = window.confirmed,
        keys = window.keys,
        take_back = window.take_back,
    },
}
