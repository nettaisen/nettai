--!strict
-- What EXE5's buster shots share (@exelib/buster: EXE6's code) but the arm they raise, EXE5's own (lib/arm): the muzzle
-- flash (EXE5's attachment row 0x0D, EXE6's 5: 0x080EBA08, sprite 0C-06 in EXE5's own colors; a spread shot's would be row
-- 0x39, 0C-59, which no EXE5 buster fires), the arm looks EXE6's code keeps (EXE5's rows 0x23 and 0x2B), EXE5's recovery
-- table (rules/buster: EXE6's values), EXE5's own mask for the panels that stop a shot's count, and EXE5's attachment.

local arm = require("./arm")
local attachment = require("../objects/attachment")
local common = require("@exelib/buster")

local buster = common.make {
    flash = attachment.look { sprite = asset.sprite("muzzle-flash"), palette = 0, attach_point = 0x10 },
    arm = attachment.look { sprite = asset.sprite("buster-arm"), palette = 0 },
    navi_arm = attachment.look { sprite = asset.sprite("navi-arm"), palette = 0 },
    recovery = require("../rules/buster").recovery,
    -- What stops a shot's panel count (0x0800D99A: one mask whichever side shoots, where EXE6's goes by the side):
    -- bodies, neutral objects, blockers and reservations, and both sides' barrier bits, the shooter's own too (a
    -- capsule of its own sitting in its row).
    closed = { [0] = 0x0F880080, [1] = 0x0F880080 },
    attachment = attachment.kind,
}

-- The arm EXE5's shots raise is EXE5's own (0x080EBAAE and 0x080EBABE: lib/arm), not EXE6's choices: an enemy navi's
-- arm and an auto-battling navi's buster arm by its AI index, MegaMan's by his soul and NaviStats +0x4C.
buster.raise_arm = arm.raise
function buster.raise_arm_for(navi: Object, holder: Object, which: Slot, while_dimmed: boolean)
    arm.raise_for(navi, which, while_dimmed, holder)
end

return buster
