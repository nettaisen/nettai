--!strict
-- The patch cards' Sbustr (`megaman/rapid-buster`, B↓Sbustr, Team ProtoMan's card 111): weapon routine 0x68
-- (0x0800FE2E, calling 0x0800FB32, BN6's `sub_8012124`: the same code but for the action's number) and action 0x2E
-- (0x080ED9B8): the shared rapid buster (@common/megaman/rapid_buster) with BN5's buster parts, projectile,
-- attachment and charged shot (row 6), its form overlay reloading its animation at its next step as it restarts
-- (0x080C374E). Nine charged shots, 5 damage for each point of Attack plus one.

local attachment = require("../../../objects/attachment/attachment")
local buster = require("../../../lib/buster")
local common = require("@common/megaman/rapid_buster")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")

local made = common.make {
    id = "bn5:megaman/rapid-buster/action",
    buster = buster,
    projectiles = projectile,
    attachment = attachment.kind,
    shot_sound = asset.sound("bn6:buster-shot"),
    shot = variants.charged,
    overlay_steps = false,
}

return define.weapon {
    id = "bn5:megaman/rapid-buster",
    -- The charge table's row 0x68, by Charge 0 to 5: past 4 it reads on into row 0x69's first entry.
    charge_ticks = { 110, 100, 90, 80, 70, 140 },
    setup = made.setup,
}
