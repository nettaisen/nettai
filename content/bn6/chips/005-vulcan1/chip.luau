--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x17 runs this
-- module, which runs the chip's own action (chips/vulcan) picked by the
-- subtype. Records reach it where something names a chip by number: the
-- Cross special's SuprVulc (the ruleset's table of numbers). It goes when
-- that is a role (step 5, phase C).

local vulcan = require("../vulcan/action")
local vulcans = require("../vulcan/chips")

local legacy = {
    state = vulcan.STATE,
}

function legacy.update(me: Object, s: any)
    local v = me.variant
    local chip = vulcans[v + 1]
    if not chip then
        error(string.format("vulcan subtype %#x reads past dword_80EBFEC", v))
    end
    (chip.action :: Action).update(me, s)
end

return legacy
