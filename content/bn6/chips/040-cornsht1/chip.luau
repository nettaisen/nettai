--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x2A runs this
-- module, which runs the chips' own action (chips/cornsht; the three are
-- alike). Records reach it where something names a chip by number:
-- CornSht1-3 (CornFsta's Program Advance). It goes when those are
-- definitions (step 5, phase C).

local cornsht = require("../cornsht/chips")

local by_number = {
    state = cornsht.STATE,
}

function by_number.update(me: Object, s: any)
    cornsht.actions.cornsht1.update(me, s)
end

return by_number
