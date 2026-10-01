--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x12 runs this
-- module, which runs the chip's own action (chips/minibomb,
-- chips/energbom, ...) picked by the subtype (and FlshBom's level, its
-- first attack parameter) as the original's `off_80EB6F8` picks the
-- thrower (and LilBoiler's level, also its first parameter). A chip
-- content defines runs its action without it; records still reach it
-- where something names a chip by number: the pack's PoisSeed (PoisPhar's
-- recipe names it), MiniBomb, EnergBom and MegEnBom when the Cross special
-- picks them (the ruleset's table of numbers), and the test content's
-- numbered bombs. It goes when those are definitions or roles (step 5,
-- phase C).

local throw = require("../../lib/bombs/throw")
local minibomb = require("../minibomb/chip")
local bigbomb = require("../bigbomb/chip")
local energbom = require("../energbom/chips")
local flshbom = require("../flshbom/chips")
local blkbomb = require("../blkbomb/chip")
local bugbomb = require("../bugbomb/chip")
local grasseed = require("../grasseed/chip")
local iceseed = require("../iceseed/chip")
local poisseed = require("../poisseed/chip")
local lilbolr = require("../lilbolr/chips")
local vdoll = require("../vdoll/chip")

local by_number = {
    state = throw.STATE,
}

-- The chips' actions by subtype (EnergBom's and MegEnBom's are the same).
local BY_SUBTYPE: { [number]: Action } = {
    [0x0] = minibomb.action,
    [0x1] = energbom[1].action,
    [0x6] = blkbomb.action,
    [0x7] = bugbomb.action,
    [0x8] = vdoll.action,
    [0x9] = iceseed.action,
    [0xC] = poisseed.action,
    [0xD] = grasseed.action,
    [0xF] = bigbomb.action,
}
local LIL_BOILER, FLASH_BOMB = 0x3, 0xE
-- LilBoiler's chips by level (0, 2, 3).
local LIL_BOILERS: { [number]: Action } = { [0] = lilbolr[1].action, [2] = lilbolr[2].action, [3] = lilbolr[3].action }

local function action_of(me: Object): Action
    local v, level = me.variant, me:attack_param(1)
    if v == FLASH_BOMB then
        local chip = flshbom[level + 1]
        if not chip then
            error(string.format("FlashBomb level %d has no chip", level))
        end
        return chip.action
    elseif v == LIL_BOILER then
        local a = LIL_BOILERS[level]
        if not a then
            error(string.format("LilBoiler level %d has no chip", level))
        end
        return a
    end
    local a = BY_SUBTYPE[v]
    if not a then
        error(string.format("bomb subtype %#x reads past off_80EB6F8", v))
    end
    return a
end

function by_number.update(me: Object, s: any)
    action_of(me).update(me, s)
end

return by_number
