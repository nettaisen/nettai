--!strict
-- Weapon routines 0x3C (`sub_8012194`) and 0x8B (`sub_8012586`, the same):
-- Reflector's guard that fires the buster's shot back (see ../3b-shield).

local shield = require("../3b-shield/shield")

local reflect = {}

-- `sub_8012194`, `sub_8012586`.
function reflect.setup(navi: Object): Action
    return shield.setup_counter(navi)
end

return reflect
