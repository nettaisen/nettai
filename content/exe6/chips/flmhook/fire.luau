--!strict
-- A flame of FlmHook's hook (the original's attack object #0xCA,
-- `sub_80DE7F4`): the shared flame (@exelib/flmhook/fire), EXE6's kind. It
-- swings along an arc of 160 pixels' radius around a point 120 pixels
-- behind its column and 48 down, starting at an angle and lasting as long
-- as its table says (`byte_80DE894`, `byte_80DE89C`, by kind, facing and
-- swing), leaving afterimages. It hits the two panels ahead of where it
-- is, only on the tick it enters a panel; a hit flashes the screen. Each
-- chip states its flames (`fire.variant`, the original's rows by Param2):
-- BigHook's are big ones, faster, another palette and shader, and they run
-- while dimmed. See docs/engine/chips.md §3.6.35.

local collision = require("../../rules/collision")
local common = require("@exelib/flmhook/fire")
local trajectory = require("../../lib/trajectory")

-- Where the arc's center is behind, 16.16; a column's width.
local CENTER_BACK = 0x78_0000
local COLUMN = 0x28_0000

local made = common.make({
    id = "flmhook/fire",
    sprite = asset.sprite("flame-hook-fire"),
    sound = asset.sound("flame-hook-fire"),
    self_type = collision.breaking,
    target = collision.hits_navis,
    -- 120 pixels behind, then (`sub_80DE8A4`) a column further for each
    -- its column is past the one it would be.
    center_x = function(me: Object, _swing: number, turn: number): number
        return -me.facing * CENTER_BACK + (me.panel_x - (4 - turn)) * COLUMN
    end,
    sets_look = true,
    hit_shakes = false,
}, trajectory.sine)

return { kind = made.kind, swing = made.swing, variant = common.variant }
