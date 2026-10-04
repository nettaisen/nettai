--!strict
-- The countdown bomb (the original's attack object #0x4B, `sub_80CD8EC`)
-- the TimeBom chips set (./controller): the shared bomb
-- (@exelib/timebom/countdown), BN6's kind and look.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/timebom/countdown")
local effects = require("../../lib/effects")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")

local countdown = {}

countdown.make_kind = common.make_kind
countdown.variant = common.variant
countdown.spawn_kind = common.spawn_kind

local BN6_SPRITE = asset.sprite("countdown-bomb")

countdown.kind = common.make_kind("timebom/countdown", {
    sprite = BN6_SPRITE,
    sounds = {
        place = asset.sound("appear"),
        tick = asset.sound("tick"),
        last = asset.sound("last"),
        blast = asset.sound("burst"),
        -- The puff where it was broken.
        puff = asset.sound("hit-bomb-1"),
    },
    blast = { is = collision.attack, hits = collision.hits_navis },
    explosion = effects.explosion,
    puff = effects.puff,
    bursts = panel_bursts.kind,
    -- How it looks absorbed (the original's obstacle kind 2): its own
    -- sprite, with the sprite parts it hid (its digits); thrown, its parts
    -- keep their own facing.
    absorbed = absorbed_obstacle.look {
        sprite = BN6_SPRITE,
        keeps_parts = true,
        own_facing = true,
    },
})

-- BN6's bomb.
function countdown.spawn(variant: CountdownVariant, x: number, y: number, side: number, damage: number, bonus: number): Object?
    return common.spawn_kind(countdown.kind, variant, x, y, side, damage, bonus)
end

return countdown
