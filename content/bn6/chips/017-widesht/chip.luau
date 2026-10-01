--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x30 runs this
-- module, which runs the chip's own action (chips/widesht) by the subtype
-- (WideSht's records send wave kind 3, SuprSpr's 6). Records reach it
-- where something names a chip by number: WideSht and SuprSpr (SuprSpr's
-- Program Advance). It goes when those are definitions (step 5, phase C).

local widesht = require("../widesht/chips")

local by_number = {
    state = widesht.STATE,
}

-- The chips' actions by subtype.
local BY_SUBTYPE: { [number]: Action } = {
    [0] = widesht.actions.widesht,
    [1] = widesht.actions.suprspr,
}

function by_number.update(me: Object, s: any)
    local v = me.variant
    local a = BY_SUBTYPE[v]
    if not a then
        -- Subtype 2 raises attachment row 0x37, past the table; others
        -- read past `byte_80ED5BC`.
        error(string.format("wide shot subtype %#x: no chip has it (byte_80ED5BC)", v))
    end
    a.update(me, s)
end

return by_number
