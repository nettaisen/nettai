--!strict
-- Spreadr1-3: a bullet from the panel ahead that bursts over the 3x3
-- around what it finds (action.luau). (SpoutCross's charged shot composes
-- the same action with its water cannon: navis/megaman/forms/
-- spoutcross/charge.)
--
-- The Spreaders' records stay the pack's for now (H-Burst's Program
-- Advance recipes name them by number): these are their actions, which
-- registration by number runs (docs/design/content-model-v2.md §5.1, "As
-- built").

local bullet = require("../../objects/bullet/bullet")
local spreader = require("./action")

-- 16 pixels up (`dword_80ECC5C`'s rows 0, 1 and 3).
local HIGH = 0x10_0000

local function spreadr(id: string): Action
    return spreader.action { id = id, gun = spreader.guns.spreader, z = HIGH, bullet = bullet.variants.spreader }
end

return {
    spreadr1 = spreadr("spreadr1/action"),
    spreadr2 = spreadr("spreadr2/action"),
    spreadr3 = spreadr("spreadr3/action"),
}
