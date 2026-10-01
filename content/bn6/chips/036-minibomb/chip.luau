--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x12 runs this
-- module, which runs the chip's own action (chips/minibomb,
-- chips/energbom, ...) picked by the subtype (and FlshBom's level, its
-- first attack parameter) as the original's `off_80EB6F8` picks the
-- thrower. A chip content defines runs its action without it; records
-- still reach it where something names a chip by number: the pack's
-- PoisSeed (PoisPhar's recipe names it), LilBoiler and VDoll (not ported),
-- MiniBomb, EnergBom and MegEnBom when the Cross special picks them (the
-- ruleset's table of numbers), and the test content's numbered bombs. It
-- goes when those are definitions or roles (step 5, phase C).

local bomb = require("../../lib/bombs/bomb")
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

local by_number = {
    state = throw.STATE,
}

-- LilBoiler's and VDoll's throws, not ported: they wind up as the original
-- does, and fail when they would throw.
local function not_ported(name: string): Action
    return throw.action {
        held = bomb.held,
        thrower = {
            throw = function(_: Object, _: Vec3, _: number): Object?
                error(name .. " is not ported yet")
            end,
        },
    }
end

-- The chips' actions by subtype (EnergBom's and MegEnBom's are the same).
local BY_SUBTYPE: { [number]: Action } = {
    [0x0] = minibomb.action,
    [0x1] = energbom[1].action,
    [0x3] = not_ported("LilBoiler"),
    [0x6] = blkbomb.action,
    [0x7] = bugbomb.action,
    [0x8] = not_ported("VDoll"),
    [0x9] = iceseed.action,
    [0xC] = poisseed.action,
    [0xD] = grasseed.action,
    [0xF] = bigbomb.action,
}
local FLASH_BOMB = 0xE

local function action_of(me: Object): Action
    local v = me.variant
    if v == FLASH_BOMB then
        local chip = flshbom[me:attack_param(1) + 1]
        if not chip then
            error(string.format("FlashBomb level %d has no chip", me:attack_param(1)))
        end
        return chip.action
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
