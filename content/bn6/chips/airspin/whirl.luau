--!strict
-- A seeking whirlwind (attack object #0xD4, `sub_80DFFB8`): the shared seeking whirlwind (@exelib/airspin/whirl,
-- BN5's too) in BN6's look: what AirSpin's variant 1 sends (no chip in the table does: `whirl.sender`; the patch card
-- bass-cross-megaman's charged shot). An enemy navi invisible or submerged it doesn't see.

local collision = require("../../rules/collision")
local common = require("@exelib/airspin/whirl")

return common.make {
    id = "airspin/whirl",
    sprite = asset.sprite("bass-anly-shot"),
    sound = asset.sound("bass-anly-shot"),
    attack = collision.attack,
    hits = collision.hits_navis,
    hidden = { "invisible", "submerged" },
}
