--!strict
-- The patch cards' triple buster (`patch-cards/triple-buster`, the original's
-- weapon routine 0x8F, `sub_8012612`; Bass BX's and the Bass Cross card's B
-- button): MegaMan's buster, with the attack's variant 1, so that its shot is
-- the spread (two more shots a row up and down). A blank or a charged shot
-- the NaviCust's bugs make instead stays one (their actions don't read the
-- variant).

local buster = require("../../navis/megaman/weapons/buster")

-- The buster's shot (its action).
local SHOT: Action = buster.shot :: any

-- `sub_8012612`.
local function setup(navi: Object): Action
    local action: Action = buster.setup(navi)
    if action == SHOT then
        (navi:action_state(SHOT) :: { mode: string }).mode = "spread"
    end
    return action
end

return define.weapon {
    id = "patch-cards/triple-buster",
    -- The charge table's row 0x8F (the buster isn't charged), by Charge 0
    -- to 7: past 4 it reads on into the next routine's row.
    charge_ticks = { 0, 0, 0, 0, 0, 130, 120, 110 },
    setup = setup,
}
