--!strict
-- Registration by number (docs/design/content-model-v2.md §12): VarSwrd's
-- record, which names the original's action 0x53, runs this module, which
-- runs the chip's own action (chips/varswrd). It goes when the record is a
-- definition: when SlashCross's A-charge (weapon 0x11) asks the chip for
-- its charge rather than its action's number, and the Beast rush's chain
-- knows the variable swords by a trait (phase C).

local vari = require("../../lib/swords/vari")
local varswrd = require("../varswrd/chip")

return {
    state = vari.STATE,
    update = function(me: Object, s: any)
        varswrd.action.update(me, s)
    end,
}
