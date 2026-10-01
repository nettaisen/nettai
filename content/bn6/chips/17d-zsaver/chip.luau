--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x5B runs this
-- module, which runs the chip's own action (chips/z-saver). Weapon 0x6E
-- starts it from ZSaver's record by number. It goes when that weapon is a
-- definition (step 5, phase C).

local zsaver = require("../z-saver/chips")

local by_number = {
    state = zsaver.STATE,
}

function by_number.update(me: Object, s: any)
    zsaver.action.update(me, s)
end

return by_number
