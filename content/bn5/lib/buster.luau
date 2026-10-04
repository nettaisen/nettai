--!strict
-- What BN5's buster shots share (@common/buster: BN6's code) but the arm they raise, BN5's own (lib/arm): BN5's muzzle
-- flash (its attachment row 0x0D, 0x080EBA08: BN6's sprite 0C-06 in BN5's own colors; a spread shot's would be row 0x39,
-- 0C-59, which no BN5 buster fires), the arm looks BN6's code keeps, BN5's recovery table (rules/buster: BN6's values)
-- and BN5's attachment.

local arm = require("./arm")
local attachment = require("../objects/attachment/attachment")
local common = require("@common/buster")

local buster = common.make {
    flash = attachment.look { sprite = asset.sprite("bn5:muzzle-flash"), palette = 0, attach_point = 0x10 },
    arm = attachment.look { sprite = asset.sprite("bn6:buster-arm"), palette = 0 },
    navi_arm = attachment.look { sprite = asset.sprite("bn6:elec-coil"), palette = 0, attach_point = 0x0E },
    recovery = require("../rules/buster").recovery,
    attachment = attachment.kind,
}

-- The arm BN5's shots raise is BN5's own (0x080EBAAE and 0x080EBABE: lib/arm), not BN6's choices: an enemy navi's
-- arm and a computer navi's buster arm by its AI index, MegaMan's by his soul and NaviStats +0x4C.
buster.raise_arm = arm.raise
function buster.raise_arm_for(navi: Object, holder: Object, which: Slot, while_dimmed: boolean)
    arm.raise_for(navi, which, while_dimmed, holder)
end

return buster
