--!strict
-- EXE5's soul's image (actor object 0x2A, 0x080C0394; spawned by 0x080C0524, EXE6's `sub_80C02A6`, which spawns its
-- own kind 0x28 for battle mode 9: EXE5's code, @exelib/souls/image): the navi MegaMan unites with comes in on a spiral
-- around him for 22 ticks (tinted), blinks for 30 and fades out over 8, two pixels up and on from him (0x080C04F2), its
-- first tick as it starts. Its look is its soul's own, the soul's form's `image` (a `SoulImage`): its sprite, the
-- soul's navi's (category 8, by the soul), and what it puts on as it comes (0x0800ED0E with 1: 0x0800EDBC's routines
-- by soul, their parts stepping even while paused) and takes off as it goes (0x0800EEBE with 1): GyroMan's propeller,
-- NapalmMan's cannon, Colonel's cape and KnightMan's ball and chain (the body overlay rows 0, 4, 3 and 1:
-- `image.wearing`), and NumberMan's face (NumberSoul's layer, row 0: 0x0800EE6E, 0x0800EFFA).

local common = require("@exelib/souls/image")
local SINE: { number } = require("../math").sine

local image = {}

image.look = common.look
image.wearing = common.wearing

local made = common.make {
    id = "soul-image",
    pool = "actor",
    sound = asset.sound("cross-merge"),
    sine = SINE,
    offset = Vec3.new(0, 0x2_0000, 0x2_0000),
    ticks_at_init = true,
    side_only = false,
    runs_while_paused = true,
}

image.kind = made.kind

-- 0x080C0524: the image of the soul `soul` (its form) for `owner`, on its side and facing.
--
-- (It is spawned at the registers 0x0801201E's read of the side's transform record leaves (0x08012402): Chaos Unison
-- (+1), the turn's arm chip by number (+6: 0xFFFF without one, which is what this has whatever the chip; a replay's
-- comparison knows the number, exe5-compat) and the turns (+3), which its first tick overwrites but NumberMan's face,
-- spawned before it, copies.)
function image.spawn(owner: Object, soul: FormDef): Object?
    local turns, chaos = owner:form_change_terms()
    return made.spawn(owner, soul, Vec3.new(if chaos then 1 else 0, 0xFFFF, turns))
end

return image
