--!strict
-- Roll, the navi of the Roll chips (the original's actor object #0x2C,
-- `sub_80C07BC`), brought by their navi chip controller: BN6's
-- (@exelib/roll/navi, which BN5's shares). She fades in over her user's
-- panel; warps up and down in front of the nearest enemy navi ahead;
-- whips it three times (a hit region 10 ticks apart, with her ribbon
-- out); warps up and away; warps her user back in herself; and drops a
-- heart on her user's panel (./heart) that heals three times her damage.
-- Then she goes, and the controller moves on. See docs/engine/chips.md
-- §3.6.20.

local common = require("@exelib/roll/navi")
local attachment = require("../../objects/attachment/attachment")
local heart = require("./heart")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

return common.make {
    id = "bn6:roll/navi",
    sprite = asset.sprite("bn6:roll-heart"),
    appear = asset.sound("bn6:appear"),
    warp = asset.sound("bn6:roll"),
    swish = asset.sound("bn6:roll-2"),
    sparkle = heart.sparkle,
    -- Her ribbon (the original's attachment row 0x12), its animation 5,
    -- animating while dimmed.
    ribbon = attachment.look { sprite = asset.sprite("bn6:roll-heart"), palette = 0 },
    ribbon_anim = 5,
    attach = attachment.attach,
    heart = heart.spawn,
    spawner_address = 0x080C0DD9,
    hits = collision.hits_navis,
    hit_type = collision.attack,
    spark = sparks.plain,
}
