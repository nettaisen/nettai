--!strict
-- SandRing (EXE4 chip 0x02F; its record at 0x080197EC + 0x2F * 0x2C): the ring throw, action 0x47 (0x080EDC5A, lib/rings),
-- as its parameter (3) has it: the navi throws (animation 0x12, a counter window) holding nothing, and 6 ticks on a sand
-- ring (../objects/ring, row 3: the ring 0c-3a, 10 pixels a tick, hit modifier 1, no status, the spreader's sound,
-- 0x8D) flies from the panel ahead, 26 pixels up, with its damage word; its hit leaves sand (type 10, the pitfall);
-- 11 ticks, then 11 more, and back to idle (animation 0).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local ring = require("../objects/ring")
local rings = require("../lib/rings")

local sandring: Chip = {
    action = rings.action {
        id = "sandring/action",
        anim = 0x12,
        held = nil,
        ring = ring.variant {
            sprite = asset.sprite("sand-ring"),
            speed = 10,
            status = nil,
            sound = asset.sound("spreader"),
            hit_mod = 1,
            sand = "pitfall",
        },
    },
    codes = { "C", "R", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 8,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { sandring = sandring }
