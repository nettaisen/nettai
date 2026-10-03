--!strict
-- What BN5's buster shots share (@common/buster: BN6's code): BN6's muzzle flash, arm and enemy navi's arm (their
-- looks), BN5's recovery table (rules/buster: BN6's values) and BN5's attachment.

local attachment = require("../objects/attachment/attachment")
local common = require("@common/buster")

return common.make {
    flash = attachment.look { sprite = asset.sprite("bn6:muzzle-flash"), palette = 0, attach_point = 0x10 },
    arm = attachment.look { sprite = asset.sprite("bn6:buster-arm"), palette = 0 },
    navi_arm = attachment.look { sprite = asset.sprite("bn6:elec-coil"), palette = 0, attach_point = 0x0E },
    recovery = require("../rules/buster").recovery,
    attachment = attachment.kind,
}
