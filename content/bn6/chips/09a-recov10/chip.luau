--!strict
-- Recov10 to Recov300 and DrkRecov: action 0x20 (`sub_80EC844`).
--
-- In the tick after the chip is used: heal the chip's HP (its own data,
-- `data.chips[id].recovery`) through the ruleset's heal, which an
-- opponent's armed AntiRecv turns into damage; count the recovery in the
-- side's statistics, and back to idle.

local recovery = {}

-- The side statistic counting recoveries (`sub_800AB46`).
local RECOVERIES = 5

-- The chip's HP.
local function amount(me: Object): number
    local chip = data.chips[me.chip]
    local hp = chip and chip.recovery
    if not hp then
        error(string.format("chip %#x runs recovery without its HP", me.chip))
    end
    return hp
end

-- `sub_80EC844`. (The heal's result, whether the trap sprang, is tested
-- and ignored.)
function recovery.update(me: Object, _s: any)
    me:heal(amount(me), true)
    battle.bump_side_stat(me.alliance, RECOVERIES, 1)
    me:exit_attack()
end

return recovery
