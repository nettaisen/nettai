--!strict
-- PrixPowr (EXE4 chip 0x138): an e-Reader card's chip. Its record and its behavior are in every ROM; its name, its
-- description, its picture and the picture's palette are the save's card data (the record points at RAM: the
-- extractor's exe4/cards.rs), the palette here as the cards give it, as EXE5's e-Reader chips' and EXE6's gift chips'
-- are.
--
-- The dimming chips' action 0x0C, its variant 0x48 (0x080EAA64): a navi chip that brings three navis one after the
-- other. Its controller (effect object #0x7A, 0x080EA900) is the navi chips' (the dim, AntiNavi, the name, the
-- undim) with a course of its own (0x080EA948):
--
-- - the user warps out; 11 ticks; VideoMan on the user's panel (./videoman), who projects LaserMan's silhouette on
--   the panel ahead;
-- - 31 ticks after he left: LaserMan on that panel (./laserman), who fires his laser along the row, if the panel is
--   solid with nothing on it; else neither he nor KendoMan comes;
-- - 6 ticks after he left: KendoMan from behind the user's side's edge in the user's row (./kendoman), who rushes
--   across the field;
-- - 6 ticks after he left (or after LaserMan didn't come): the undim, the user warping back in as it starts.
--
-- Each comes with the chip's damage word and bonus (VideoMan hits nothing).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs, its palette from the
-- card data in Tango's save templates.

local common = require("@exelib/navi_chips/controller")
local kendoman = require("@self/kendoman")
local laserman = require("@self/laserman")
local navi_chips = require("../lib/navi_chips")
local videoman = require("@self/videoman")

local SOLID = 0x10
-- What LaserMan's panel must be free of (0x080EA9CA): a body or an object.
local IN_THE_WAY = 0x0F80_0000

local VIDEO, LASER, KENDO, BACK = 0x0, 0x4, 0x8, 0xC

-- Count the timer down: true once it fell below 0.
local function ran_out(c: Object): boolean
    local left = c.timer - 1
    c.timer = int.u16(left)
    return left < 0
end

-- 0x080EA948, by phase (0x080EA95C).
local function course(c: Object)
    local phase = c.phase
    if phase == VIDEO then
        -- 0x080EA96C.
        if c.phase_init ~= 1 then
            navi_chips.warp(c.alliance, true)
            c.phase_init = 1
            c.timer = 10
        end
        if not ran_out(c) then
            return
        end
        if videoman.summon(c, c.panel_x, c.panel_y, common.damage(c)) then
            common.await(c)
        end
        c.phase, c.phase_init = LASER, 0
        c.timer = 30
    elseif phase == LASER then
        -- 0x080EA9AE.
        if common.waiting(c) or not ran_out(c) then
            return
        end
        local x = c.panel_x + c.facing
        if field.check(x, c.panel_y, SOLID, IN_THE_WAY) then
            if laserman.summon(c, x, c.panel_y, common.damage(c)) then
                common.await(c)
            end
            c.phase, c.phase_init = KENDO, 0
        else
            c.phase, c.phase_init = BACK, 0
        end
        c.timer = 5
    elseif phase == KENDO then
        -- 0x080EAA0C.
        if common.waiting(c) or not ran_out(c) then
            return
        end
        if kendoman.summon(c, c.alliance * 7, c.panel_y, common.damage(c)) then
            common.await(c)
        end
        c.phase, c.phase_init = BACK, 0
        c.timer = 5
    elseif phase == BACK then
        -- 0x080EAA42.
        if common.waiting(c) or not ran_out(c) then
            return
        end
        common.undim(c)
        navi_chips.warp(c.alliance, false)
    else
        error(string.format("PrixPowr's controller: effect phase %#x reads past its table (0x080EA95C)", phase))
    end
end

local CONTROLLER = navi_chips.controller {
    id = "prixpowr/controller",
    name = "PrixPowr",
    course = course,
}

local chip: Chip = {
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 94,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    art_palette = { 0x0000, 0x4645, 0x7B2B, 0x1C53, 0x3D3B, 0x51BD, 0x625E, 0x0842, 0x1CE7, 0x2928, 0x2D8C, 0x3DEF, 0x4E73, 0x6AF9, 0x777C, 0x7BDE },
    dimming = CONTROLLER.hook,
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { prixpowr = chip }
