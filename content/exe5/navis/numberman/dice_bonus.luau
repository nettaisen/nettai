--!strict
-- NumberMan's DiceBons (EXE5 DS's party battle: 0x02004CC8, from the chip attack's end 0x0200634C, `sub_801171C`'s
-- counterpart; docs/design/exe5ds-map.md §4.7), the rules' `chip_ended`: as a navi's chip attack ends, a side that
-- passes the light test draws its chance (RNG2, every such end while the battle goes on); then, once a battle, with
-- NumberMan a benched member who may support (offline, side 0's alone), a chip without dimming just used and a chip that
-- deals damage next, he rolls his dice.

local party = require("../../rules/party")
local numberman = require("../numberman").numberman

local dice_bonus = {}

-- RNG2 bits 10–13 at most this: 7 in 16.
local CHANCE = 6

local function has_flag(chip: Chip, flag: string): boolean
    return chip.flags ~= nil and table.find(chip.flags, flag) ~= nil
end

-- 0x02004CC8: whether the dice roll.
local function rolls(side: number): boolean
    if not party.has_partners(side) or not party.light(side) or battle.over() then
        return false
    end
    if bit32.band(bit32.rshift(battle.rng(), 10), 0xF) > CHANCE then
        return false
    end
    if party.dice_bonus_spent(side) then
        return false
    end
    if not battle.link() and side ~= 0 then
        return false
    end
    if not party.may_support(side, numberman) or not party.benched(side, numberman) then
        return false
    end
    if battle.navi(side).navi == numberman then
        return false
    end
    -- The hand's cursor is past the chip just used.
    local cursor = battle.hand_cursor(side)
    if cursor == 0 then
        return false
    end
    local used = battle.hand_chip(side, cursor - 1)
    if used == nil or has_flag(used, "dimming") then
        return false
    end
    local next = battle.hand_chip(side, cursor)
    if next == nil or not has_flag(next, "has_damage") then
        return false
    end
    party.spend_dice_bonus(side)
    return true
end

-- The rules' `chip_ended`.
function dice_bonus.chip_ended(side: number, _navi: Object)
    if rolls(side) then
        -- TODO(party battle, the supports): the dimming, telop chip 0x189 and T4 0x95 (0x022264DC).
    end
end

return dice_bonus
