--!strict
-- Registration by number (docs/design/content-model-v2.md §12): MoonBld's
-- record, which names the original's action 0x40, runs this module, which
-- runs the chip's own action (chips/moonbld). It goes when the record is a
-- definition: when SlashCross's A-charge (weapon 0x11) asks the chip for
-- its charge rather than its action's number, and the Cross special names
-- chips by handle (phase C).

local action = require("../moonbld/action")
local moonbld = require("../moonbld/chip")

return {
    state = action.STATE,
    update = function(me: Object, s: any)
        moonbld.action.update(me, s)
    end,
}
