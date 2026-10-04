--!strict
-- BigHook's hook (BN5's effect object #0x8C, 0x080EA8A0): the shared hook (@exelib/flmhook/hook), BN5's kind, the
-- BigHook chip's navi (the summon table's entry 26, 0x080EA990). It has no sprite; it swings two flames (./fire),
-- both from column 5 (side 1: 2; 0x080EA8F4), the first from row 4 and 20 ticks later the second from row 0, then
-- goes 10 ticks later, and the controller moves on. Its spawner leaves its element 0, and its flames' second
-- parameter 0: read with their swing as one halfword, it turns them the way the swing alone says.

local common = require("@exelib/flmhook/hook")
local fire = require("./fire")
local sparks = require("../../lib/sparks")

-- Its flames' column (0x080EA8F4: 2 + 3 * (1 - (side ^ flip))).
local function column(me: Object): number
    return 2 + 3 * bit32.bxor(bit32.bxor(me.alliance, me.flip), 1)
end

local made = common.make {
    id = "bn5:bighook/hook",
    columns = function(me: Object, _big: boolean): (number, number)
        local x = column(me)
        return x, x
    end,
    flame = function(owner: Object, x: number, y: number, element: number, damage: number, swing: number, flames: { big: boolean, spark: Spark }): Object?
        return fire.swing(owner, x, y, element, damage, { swing = swing, big = false, spark = flames.spark })
    end,
    keeps_element = false,
}

local hook = {}

hook.kind = made.kind

-- BigHook's navi hook (0x080EA990, which the controller calls through r3): the hook, its flames breaking, reporting
-- to the controller.
hook.summon = made.summon({ big = true, spark = sparks.breaking }, 0x080EA991)

return hook
