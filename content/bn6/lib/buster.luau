--!strict
-- What BN6's buster shots share (the buster, the charged shot, the blank
-- shot, DustCross's charged shot and the like; @common/buster): the arm the
-- navi raises, the recovery after a shot, and the move that may cut the
-- recovery short. See docs/engine/objects-and-player.md §B6.

local attachment = require("../objects/attachment/attachment")
local common = require("@common/buster")

return common.make {
    -- The muzzle flash a shot leaves in the navi's first related slot (the
    -- original's attachment row 5, at attach point 0x10), and the buster
    -- arm (row 6), which `raise_arm` raises and a thrown obstacle leaves
    -- there instead.
    flash = attachment.look { sprite = asset.sprite("bn6:muzzle-flash"), palette = 0, attach_point = 0x10 },
    arm = attachment.look { sprite = asset.sprite("bn6:buster-arm"), palette = 0 },
    -- An enemy navi's arm (the original's attachment row 0x2B, at attach
    -- point 0x0E).
    navi_arm = attachment.look { sprite = asset.sprite("bn6:elec-coil"), palette = 0, attach_point = 0x0E },
    -- The buster's recovery in ticks (`byte_80209CC`), by Rapid, then the
    -- open panels ahead.
    recovery = require("../rules/buster").recovery,
    attachment = attachment.kind,
}
