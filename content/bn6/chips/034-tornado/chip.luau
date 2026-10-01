--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x2F runs
-- this module, which runs the chip's own blow (chips/tornado) picked by
-- the subtype as `sub_80ED454` branches. Tornado and Static are
-- definitions; the record that still reaches it is Tornado's, which the
-- Cross special picks by number (berserk.rs `CROSS_SPECIAL_CHIPS`). It goes
-- when that list names chips (phase C, step 10).

local tornado = require("../tornado/chips")

local by_number = {
    state = tornado.STATE,
}

-- The blows by subtype (3: the action's third form, which no chip has).
local BY_SUBTYPE: { [number]: Action } = {
    [1] = tornado[1].action,
    [2] = tornado[2].action,
    [3] = tornado.back_spread,
}

function by_number.update(me: Object, s: any)
    local subtype = me.variant
    local a = BY_SUBTYPE[subtype]
    if not a then
        error(string.format("Tornado subtype %#x has no blow (sub_80ED454)", subtype))
    end
    a.update(me, s)
end

return by_number
