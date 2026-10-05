--!strict
-- A flame of BigHook's hook (EXE5's attack object #0xCA, 0x080DF9D4): the shared flame (@exelib/flmhook/fire), EXE5's
-- kind: EXE6's big flame's arcs and turn (0x080DFA7C), its sprite 0C-56, breaking, hitting navis; no afterimages,
-- its facing and palette left as loaded, its arc's center 120 pixels behind (the second flame's 80: 0x080DFA74),
-- not from its column; a hit shakes the camera too (0x080DFA9A); it always runs while dimmed.

local collision = require("../../rules/collision")
local common = require("@exelib/flmhook/fire")
local trajectory = require("../../lib/trajectory")

local ARC = { 0x16, 0xD2, 0x1E, 0x1E, 0x6A, 0x8A, 0x1E, 0x1E }
-- Its arc's center behind it, by swing (0x080DFA74), 16.16.
local BACK: { [number]: number } = { [0] = 0x78_0000, 0x50_0000 }

return common.make({
    id = "bighook/fire",
    sprite = asset.sprite("flame-hook-fire"),
    sound = asset.sound("flame-hook-fire"),
    self_type = collision.breaking,
    target = collision.hits_navis,
    arcs = function(_big: boolean): { number }
        return ARC
    end,
    turn = function(_big: boolean): number
        return 0x280
    end,
    center_x = function(me: Object, swing: number, _turn: number): number
        local back = BACK[swing]
        if back == nil then
            error(string.format("BigHook's flame swing %d reads past 0x080DFA74", swing))
        end
        return -me.facing * back
    end,
    trail = nil,
    sets_look = false,
    hit_shakes = true,
    while_dimmed = function(_big: boolean): boolean
        return true
    end,
}, trajectory.sine)
