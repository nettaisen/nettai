--!strict
-- BigHamr1 to BigHamr3 and GodHammr (EXE4 chips 0x068 to 0x06A, 0x0D7): the dimming chips' variant 0x10, the hammer
-- chips' controller (./controller) setting a hammer (./hammer) in front of the user that swings down on the panel
-- ahead (each chip's own comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@self/controller")
local hammer = require("@self/hammer")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

-- 0x080D6AA0 (BigHamr's landing): on a solid panel two of the other side's solid panels crack (0x080D6B74) and the
-- camera shakes by `shake`; its hit on the panel ahead, breaking, with the breaking spark, solid or not.
local function smash(shake: number): (me: Object, x: number, y: number, solid: boolean) -> ()
    return function(me: Object, x: number, y: number, solid: boolean)
        if solid then
            hammer.crack_two(me)
            hammer.shake(shake)
        end
        hammer.hit(me, x, y, regions.single, collision.thrown_break, sparks.breaking)
    end
end

-- 0x080D6AC6 (GodHammr's landing): on a solid panel the camera shakes by 3 and a quake hits every solid panel
-- (collision type 0x26, no spark); else its hit on the panel ahead alone (thrown, the breaking spark).
local function quake(me: Object, x: number, y: number, solid: boolean)
    if solid then
        hammer.shake(3)
        hammer.hit(me, x, y, regions.solid, collision.collision_26, nil)
    else
        hammer.hit(me, x, y, regions.single, collision.thrown, sparks.breaking)
    end
end

local function big_hammer(id: string, palette: number, shake: number)
    local made = hammer.make { id = id, palette = palette, stand_anim = 1, blinks = true, land = smash(shake) }
    return made.spawn
end

-- BigHamr1 (EXE4 chip 0x068): its hammer in palette 0, shaking the camera by 1.
local bighamr1: Chip = {
    codes = { "B", "R", "O" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 0,
    mb = 22,
    damage = 160,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = controller.make("bighamr1/controller", big_hammer("bighamr1/hammer", 0, 1)),
}

-- BigHamr2 (EXE4 chip 0x069): its hammer in palette 4, shaking the camera by 2.
local bighamr2: Chip = {
    codes = { "G", "J", "W" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 1,
    mb = 33,
    damage = 220,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = controller.make("bighamr2/controller", big_hammer("bighamr2/hammer", 4, 2)),
}

-- BigHamr3 (EXE4 chip 0x06A): its hammer in palette 8, shaking the camera by 3.
local bighamr3: Chip = {
    codes = { "D", "V", "Z" },
    element = "null",
    family = "metal",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 280,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = controller.make("bighamr3/controller", big_hammer("bighamr3/hammer", 8, 3)),
}

-- GodHammr (EXE4 chip 0x0D7): its hammer in palette 10, standing still as it winds up, shaking the camera by 3, its
-- landing a quake.
local godhammr: Chip = {
    codes = { "G" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 3,
    mb = 68,
    damage = 250,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller.make(
        "godhammr/controller",
        hammer.make({ id = "godhammr/hammer", palette = 10, stand_anim = 0, blinks = false, land = quake }).spawn
    ),
}

return { bighamr1 = bighamr1, bighamr2 = bighamr2, bighamr3 = bighamr3, godhammr = godhammr }
