--!strict
-- BigHook's hook (EXE5's effect object #0x8C, 0x080EA8A0): the shared hook (@exelib/flmhook/hook), EXE5's kind, the
-- BigHook chip's navi (the summon table's entry 26, 0x080EA990). It has no sprite; it swings two flames (./fire),
-- both from column 5 (side 1: 2; 0x080EA8F4), the first from row 4 and 20 ticks later the second from row 0, then
-- goes 10 ticks later, and the controller moves on. Its spawner leaves its element 0, and its flames' second
-- parameter 0: read with their swing as one halfword, it turns them the way the swing alone says.

local common = require("@exelib/flmhook/hook")
local common_fire = require("@exelib/flmhook/fire")
local fire = require("./fire")
local sparks = require("../../lib/sparks")

-- Its flames' column (0x080EA8F4: 2 + 3 * (1 - (side ^ flip))).
local function column(me: Object): number
    return 2 + 3 * bit32.bxor(bit32.bxor(me.alliance, me.flip), 1)
end

local made = common.make {
    id = "bighook/hook",
    flame = function(owner: Object, x: number, y: number, element: number, damage: number, swing: number, flames: FlameHookFlames): Object?
        return fire.swing(owner, x, y, element, damage, { swing = swing, variant = flames.variant, spark = flames.spark })
    end,
    keeps_element = false,
}

-- Its flames: both from its column; EXE6's big flame's arcs and turn (0x080DFA7C), their second parameter 0 (read
-- with their swing as one halfword, it turns them the way the swing alone says); no afterimages, the palette left
-- as loaded; running while dimmed; breaking.
local FLAMES = common.flames {
    columns = function(me: Object): (number, number)
        local x = column(me)
        return x, x
    end,
    variant = common_fire.variant {
        swing_high = 0,
        arcs = { 0x16, 0xD2, 0x1E, 0x1E, 0x6A, 0x8A, 0x1E, 0x1E },
        turn = 0x280,
        palette = 0,
        while_dimmed = true,
    },
    spark = sparks.breaking,
}

local hook = {}

hook.kind = made.kind

-- BigHook's navi hook (0x080EA990, which the controller calls through r3): the hook, its flames breaking, reporting
-- to the controller.
hook.summon = made.summon(FLAMES, 0x080EA991)

return hook
