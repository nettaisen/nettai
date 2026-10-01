--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x18 runs
-- this module, which runs the chip's own throw (chips/yoyo,
-- chips/greatyo) picked by the subtype as `sub_80EC02A` branches. The
-- records that reach it are YoYo's (GreatYo's recipe names it by number)
-- and GreatYo's (a Program Advance). It goes when they are definitions
-- (step 5, phase C).

local greatyo = require("../greatyo/chip")
local throw = require("../yoyo/throw")
local yoyo = require("../yoyo/chip")

local by_number = {
    state = throw.STATE,
}

-- The throws by subtype.
local BY_SUBTYPE: { [number]: Action } = {
    [0] = yoyo.action,
    [1] = greatyo.action,
}

function by_number.update(me: Object, s: any)
    local subtype = me.variant
    local a = BY_SUBTYPE[subtype]
    if not a then
        error(string.format("YoYo subtype %#x has no throw (sub_80EC02A)", subtype))
    end
    a.update(me, s)
end

return by_number
