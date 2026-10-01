--!strict
-- GigaCan1-3: the Program Advances of three Cannons, HiCannons or
-- M-Cannons of one code. The cannon of the chip they're made of fires a
-- shot that bursts over the 3x3 around what it hits (and over the last two
-- columns when it leaves the field), and the cannon and the navi leave
-- afterimages as it charges (the original's action 0x14 with subtypes 4 to
-- 6; lib/cannon.luau).
--
-- Their records stay the pack's for now (a definition can't hold a
-- Program Advance's recipes yet): these are their actions, which
-- registration by number runs (docs/design/content-model-v2.md §5.1, "As
-- built").

local cannon = require("../../lib/cannon")
local variants = require("../../objects/projectile/variants")

local function giga(id: string, arm: AttachmentLook): Action
    return cannon.action { id = id, arm = arm, shot = variants.gigacan, afterimages = true }
end

return {
    gigacan1 = giga("gigacan1/action", cannon.arms.cannon),
    gigacan2 = giga("gigacan2/action", cannon.arms.hicannon),
    gigacan3 = giga("gigacan3/action", cannon.arms.m_cannon),
}
