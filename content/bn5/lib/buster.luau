--!strict
-- What BN5's buster shots share (@exelib/buster: BN6's code) but the arm they raise, BN5's own (lib/arm): the muzzle
-- flash (BN5's attachment row 0x0D, BN6's 5), the arm looks BN6's code keeps (BN5's rows 0x23 and 0x2B), BN5's recovery
-- table (rules/buster: BN6's values) and BN5's attachment.

local arm = require("./arm")
local attachment = require("../objects/attachment/attachment")
local common = require("@exelib/buster")

local buster = common.make {
    flash = attachment.look { sprite = asset.sprite("muzzle-flash"), palette = 0, attach_point = 0x10 },
    arm = attachment.look { sprite = asset.sprite("buster-arm"), palette = 0 },
    navi_arm = attachment.look { sprite = asset.sprite("navi-arm"), palette = 0 },
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
