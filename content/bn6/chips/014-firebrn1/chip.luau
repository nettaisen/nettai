--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x27 runs
-- this module, which runs the chip's own burn (chips/firebrn,
-- chips/widebrn). The records that reach it are FireBrn1 to 3 (WideBrn's
-- recipes and the Cross special name them by number) and WideBrn1 to 3
-- (Program Advances). Within each series the records differ only in their
-- damage, which the attack holds, so the first one's burn runs for each;
-- WideBrn's flame is the wide one (the bit 0x80 of the record's second
-- parameter). It goes when the records are definitions (step 5, phase C).

local burn = require("../../lib/burner/burn")
local firebrn = require("../firebrn/chips")
local widebrn = require("../widebrn/chips")

local by_number = {
    state = burn.STATE,
}

-- A wide flame's bit in the second parameter.
local WIDE = 0x80

function by_number.update(me: Object, s: any)
    local a = if bit32.btest(me:attack_param(2), WIDE) then widebrn[1].action else firebrn[1].action
    a.update(me, s)
end

return by_number
