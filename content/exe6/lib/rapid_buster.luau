--!strict
-- The rapid buster (the original's action 0x5D, `sub_80F00F2`, and its
-- weapon routine 0x39, `sub_8012124`): nine charged shots, six ticks
-- apart, each from the panel in front and a row up, down or level in an
-- order one draw picks: the shared rapid buster (@exelib/megaman/rapid_buster,
-- EXE5's too) with EXE6's buster parts, projectile, attachment and sound, its
-- form overlay stepped at once as it restarts. See docs/engine/shot-chips.md §11.
--
-- Weapon routine 0x39 is named by no form, navi or NaviStats byte; its
-- twin 0x92 (the same routine, its own charge row) is the patch card Bass
-- BX's charged shot (patch_cards/bassbx/charge), which composes `rapid_buster.setup`.

local attachment = require("../objects/attachment")
local buster = require("./buster")
local common = require("@exelib/megaman/rapid_buster")
local projectile = require("./projectile")
local variants = require("../objects/projectile/variants")

return common.make {
    id = "rapid-buster/action",
    buster = buster,
    projectiles = projectile,
    attachment = attachment.kind,
    shot_sound = asset.sound("buster-shot"),
    shot = variants.charged,
    overlay_steps = true,
}
