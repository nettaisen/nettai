--!strict
-- A flame of FlmHook's hook (the original's attack object #0xCA,
-- `sub_80DE7F4`): the shared flame (@common/flmhook/fire), BN6's kind. It
-- swings along an arc of 160 pixels' radius around a point 120 pixels
-- behind its column and 48 down, starting at an angle and lasting as long
-- as its table says (`byte_80DE894`, `byte_80DE89C`, by kind, facing and
-- swing), leaving afterimages. It hits the two panels ahead of where it
-- is, only on the tick it enters a panel; a hit flashes the screen.
-- BigHook's flames are big ones: faster, another palette and shader, and
-- they run while dimmed. See docs/engine/chips.md §3.6.35.

local collision = require("../../rules/collision")
local common = require("@common/flmhook/fire")
local trajectory = require("../../lib/trajectory")

-- Where the arc's center is behind, 16.16; a column's width.
local CENTER_BACK = 0x78_0000
local COLUMN = 0x28_0000
-- By size (`off_80DE88C`): four bytes per facing (right, left), the start
-- angle (high byte) by swing and then the lifetime by swing.
local ARCS = {
    [false] = { 0x16, 0xF5, 0x1E, 0x1E, 0x6A, 0x8B, 0x1E, 0x1E },
    [true] = { 0x16, 0xD2, 0x1E, 0x1E, 0x6A, 0x8A, 0x1E, 0x1E },
}
-- How fast its angle turns, by size (`off_80DE9DC`).
local TURN = { [false] = 0x380, [true] = 0x280 }
-- Afterimages: every 4 ticks (6 for a big one), their shader by size,
-- and how long they last.
local TRAIL = { [false] = 4, [true] = 6 }
local SHADER = { [false] = 0x83E0, [true] = 0xA1EF }

return common.make({
    id = "bn6:flmhook/fire",
    sprite = asset.sprite("bn6:flame-hook-fire"),
    sound = asset.sound("bn6:flame-hook-fire"),
    self_type = collision.breaking,
    target = collision.hits_navis,
    arcs = function(big: boolean): { number }
        return ARCS[big]
    end,
    turn = function(big: boolean): number
        return TURN[big]
    end,
    -- 120 pixels behind, then (`sub_80DE8A4`) a column further for each
    -- its column is past the one it would be.
    center_x = function(me: Object, _swing: number, turn: number): number
        return -me.facing * CENTER_BACK + (me.panel_x - (4 - turn)) * COLUMN
    end,
    trail = {
        every = function(big: boolean): number
            return TRAIL[big]
        end,
        shader = function(big: boolean): number
            return SHADER[big]
        end,
        ticks = 0xE,
    },
    sets_look = true,
    hit_shakes = false,
    while_dimmed = function(big: boolean): boolean
        return big
    end,
}, trajectory.sine)
