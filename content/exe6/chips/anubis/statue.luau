--!strict
-- The Anubis statue (the original's attack object #0x55, `sub_80CF0D0`;
-- docs/engine/dimming-chip-effects.md §7): Anubis's and PoisPhar's. It
-- drops onto the panel in front of the user; on anything but free solid
-- ground it breaks at once. Standing, it drains every enemy panel's navi
-- 1 HP each period (Anubis's 6 ticks, PoisPhar's every tick), with bubbles
-- rising over the enemy's area every 16 ticks, for its lifetime (a field
-- obstacle's, counting only ticks it runs) or until time is up. The
-- behavior is EXE5's too (@exelib/anubis/statue), made here with EXE6's look.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed_obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/anubis/statue")
local effects = require("../../lib/effects")
local rising_bubble = require("../../objects/rising_bubble")

local statue = {}

-- The drain's self type (`byte_80CF31C`'s row 0x52).
local DRAIN_IS = new.collision { side0 = 0x80001080, side1 = 0x40001080, row_offset = 0x290 }

local made = common.make {
    id = "anubis/statue",
    sprite = asset.sprite("anubis"),
    sounds = { land = asset.sound("land"), ["break"] = asset.sound("hit-bomb-1") },
    -- A field obstacle of its side, hit by the other side.
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    drain = DRAIN_IS,
    nothing = collision.nothing,
    absorbed = absorbed_obstacle.look { sprite = asset.sprite("anubis") },
    effects = { puff = effects.puff, explosion = effects.explosion },
    bubble = rising_bubble.kind,
}
statue.kind = made.kind
statue.spawn = made.spawn

-- An Anubis statue's variant (a row of `byte_80CF0C0`, by the chip's first
-- parameter): each chip's own (Anubis's, PoisPhar's).
statue.variant = common.variant

return statue
