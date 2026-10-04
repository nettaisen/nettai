--!strict
-- VDoll's doll (the original's attack object #0x7A, `sub_80D46B8`): the shared one (@exelib/vdoll/doll) in BN6's look.
-- A field object with 50 HP thrown three panels ahead in a 60-tick arc, which can hit in its last five ticks. Where
-- it lands the panel turns to poison. The first damage it takes after landing curses the other side (./curse): a
-- dimming that hits every combatant there with that damage. See docs/engine/shot-chips.md §14.2.

local absorbed_obstacle = require("../../objects/absorbed-obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/vdoll/doll")
local curse = require("./curse")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local SPRITE = asset.sprite("voodoo-doll")

return common.make {
    id = "vdoll/doll",
    sprite = SPRITE,
    -- Its identity: a field object, with the look something that takes its place shows (DustMan's junk).
    identity = define.identity {
        id = "vdoll",
        class = "field_object",
        version = 3,
        ai_index = 1,
        look = { sprite = asset.sprite("voodoo-doll"), anim = 1, palette = 0, shadow = true },
    },
    -- How it looks absorbed (the original's obstacle kind's sprite: its own).
    absorbed = absorbed_obstacle.look { sprite = SPRITE },
    -- An object of its side that the other side's hits reach (hit modifier 1); set down, an obstacle of no side that
    -- anything can hit (`sub_80D4870`: hit modifier 3).
    is = collision.own_object,
    hits = collision.hit_by_other_side_or_blockers,
    hit_mod = 1,
    spark = sparks.plain,
    set_down = { is = collision.neutral_body, hits = collision.everything, hit_mod = 3 },
    poison_sound = asset.sound("panel-poison"),
    puff = effects.puff,
    trajectory = trajectory,
    curse = curse,
}
