--!strict
-- ParaShl (IronShl's Program Advance): IronShl's throw
-- (chips/ironshl/throw.luau) of three shells down the navi's own back
-- column, which slide four panels, bump and slide back.

local ironshl_chips = require("./ironshl")
local shell = require("./ironshl/shell")
local throw = require("./ironshl/throw")

-- Its shells' speed (`byte_80CA7A0`'s third), 16.16 a tick.
local SPEED = 0x8_0000

-- Its shells (the shell's third parameter 1): three down the navi's own
-- back column, sliding four panels and never knocking back; 16 ticks
-- there, then they turn and slide back.
local PARA = shell.variant {
    throw = function(me: Object, place: (x: number, y: number) -> ())
        local x = me.alliance * 5 + 1
        for y = 1, 3 do
            place(x, y)
        end
    end,
    stop = 0x10,
    panels = 4,
    pushes = false,
    done = function(me: Object)
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
        me.flip = bit32.bxor(me.flip, 1)
        me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    end,
}

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 350,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = {
        { order = 37, code_run = { chip = ironshl_chips.ironshl1, count = 3 } },
        { order = 38, code_run = { chip = ironshl_chips.ironshl2, count = 3 } },
        { order = 39, code_run = { chip = ironshl_chips.ironshl3, count = 3 } },
    },
    action = throw.action { id = "parashl/action", shell = { palette = 0, speed = SPEED, bumps = 1, variant = PARA } },
}

return { parashl = chip }
