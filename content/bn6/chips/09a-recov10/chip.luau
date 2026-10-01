--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x20 runs
-- this module, which runs the chip's own heal (chips/recov,
-- chips/drkrecov) picked by the subtype, as `sub_80EC844` picks the HP
-- from `byte_80EC870`. Recov10 to Recov300 are definitions; the records
-- that still reach it are DrkRecov's (a dark chip) and Recov10's, which
-- the ruleset gives for a dark chip without a bug frag (chip_use.rs
-- `SUBSTITUTES`), and the test content's. It goes when dark chips name
-- their substitutes (phase C, step 10).

local drkrecov = require("../drkrecov/chip")
local heal = require("../recov/heal")
local recov = require("../recov/chips")

local by_number = {}

-- The heals by subtype: Recov10 to Recov300, DrkRecov, and the table's
-- last entry, which heals nothing (no chip has it).
local BY_SUBTYPE: { [number]: Action } = {
    [0] = recov[1].action,
    recov[2].action,
    recov[3].action,
    recov[4].action,
    recov[5].action,
    recov[6].action,
    recov[7].action,
    recov[8].action,
    drkrecov.action,
    heal.action { id = "recov/none", hp = 0 },
}

function by_number.update(me: Object, s: any)
    local a = BY_SUBTYPE[me.variant]
    if not a then
        error(string.format("recovery subtype %#x reads past byte_80EC870 (sub_80EC844)", me.variant))
    end
    a.update(me, s)
end

return by_number
