--!strict
-- What a link navi's level gives it in battle, by the level of the navi code its side's save received (exe6.navi_level:
-- the rules' `level`): a navi definition's functions of its side, which the round's setup asks once for each
-- side (the engine keeps the answers for the round, and knows no level). A link navi's chip bonus (`chip_bonus.damage`:
-- `sub_800F09E`, its row of `byte_8021300`), whether its charged chips charge (`charged_chips.when`: `sub_800F49E`,
-- from its level in `byte_8021369`), ChargeMan's Fire charge (`fire_charge`: `sub_80F0608`, `byte_802136D`). Its stats
-- at a level are the save's reload's (./levels).

local exe6 = require("./api")

local by_level = {}

-- `row`'s entry at the side's level (row[1] is level 0's), nil without a level. A level past the row is an error: the
-- original reads past its table.
function by_level.row(row: { number }): (side: number) -> number?
    return function(side: number): number?
        local level = exe6.navi_level(side)
        if level == nil then
            return nil
        end
        local v = row[level + 1]
        if v == nil then
            error(`a row of {#row} levels read at level {level}`)
        end
        return v
    end
end

-- Whether the side's level is `from` or more: false without a level.
function by_level.from(from: number): (side: number) -> boolean
    return function(side: number): boolean
        local level = exe6.navi_level(side)
        return level ~= nil and level >= from
    end
end

return by_level
