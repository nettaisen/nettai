--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x2C runs
-- this module, which runs the chip's own throw (chips/ironshl,
-- chips/parashl) picked by the record's parameters: the third marks
-- ParaShl's, the first is IronShl1 to 3's speed and palette. The records
-- that reach it are IronShl1 to 3 (ParaShl's recipes name them by number)
-- and ParaShl (a Program Advance). It goes when they are definitions
-- (step 5, phase C).

local ironshl = require("../ironshl/chips")
local parashl = require("../parashl/chip")
local throw = require("../ironshl/throw")

local by_number = {
    state = throw.STATE,
}

local function action_of(me: Object): Action
    if me:attack_param(3) ~= 0 then
        return parashl.action
    end
    local speed = me:attack_param(1)
    local chip = ironshl[speed + 1]
    if not chip then
        error(string.format("iron shell speed %d reads past byte_80CA7A0", speed))
    end
    return chip.action
end

function by_number.update(me: Object, s: any)
    action_of(me).update(me, s)
end

return by_number
