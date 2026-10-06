--!strict
-- ElemTrap's trap (the original's attack object #0x4D, `sub_80CDF84`): the
-- shared trap (@exelib/elemtrap/trap), EXE6's kind and look. The trap
-- chips' controller (lib/traps/controller) sets it for ElemTrap:
-- `trap.set(controller, user, chip, bonus)` (`sub_80E358C`).

local collision = require("../../rules/collision")
local common = require("@exelib/elemtrap/trap")
local strike = require("./strike")

local trap = {}

-- `sub_800ABC6`: the sparkle on each of the enemy navi's panels
-- (`byte_80CE0DC`: the panels of the other side's player navi), 16 pixels
-- down and 32 up from the panel's center, and its sound.
local SPARKLE = new.effect { sprite = asset.sprite("hit-sparks"), anim = 7, palette = 0 }
local SPARKLE_SOUND = asset.sound("dimming-sparkle")
local SPARKLE_DY, SPARKLE_Z = 0x10_0000, 0x20_0000

-- EXE6's look: it watches fire to wood, is nothing and reaches the other
-- side's attacks and objects, sparkles 16 pixels down and 32 up, and
-- strikes with ElemTrap's counterattack.
local EXE6: ElemTrapLook = {
    sparkle = SPARKLE,
    sparkle_sound = SPARKLE_SOUND,
    watches = "fire_to_wood",
    self_type = collision.nothing,
    target = collision.hit_by_other_attacks,
    sparkle_dy = SPARKLE_DY,
    sparkle_z = SPARKLE_Z,
    strike_kind = strike.kind,
}

-- A trap kind (`id`) of `look`, and a trap of a kind.
trap.make_kind = common.make_kind
trap.spawn = common.spawn

trap.kind = common.make_kind("elemtrap/trap", EXE6)

-- ElemTrap's (`sub_80E358C`): EXE6's trap.
function trap.set(controller: Object, user: Object, chip: Chip?, bonus: number): Object?
    return common.spawn(trap.kind, controller, user, chip, bonus)
end

return trap
