--!strict
-- The weapon routines that fire a chip, which the patch cards name for MegaMan's B button, charged shot and B+Left
-- special (NaviStats +0x09, +0x0A, +0x0C: the table at 0x0800CA7C): each loads the chip as the attack (0x0800D406,
-- EXE5's 0x0800FE78: the chip's record's damage and hit parameter, lockout, element, variant and parameters; no extra)
-- and starts the chip's use. Each weapon is its routine (`megaman/<chip>`), its charge time the charge table's row
-- (0x080180B8, by the routine) by Charge 0 to 4, Charge 5 reading on into the next row's first entry.
--
-- The rest wait on their chips (docs/design/exe4-map.md §18 item 57).

local airshot = require("../../../chips/airshot").airshot
local bubbler = require("../../../chips/bubbler").bubbler
local cannon = require("../../../chips/cannon").cannon
local crakout_chips = require("../../../chips/crakout")
local fullcust = require("../../../chips/fullcust").fullcust
local heatshot = require("../../../chips/heatshot").heatshot
local lance = require("../../../chips/lance").lance
local minibomb = require("../../../chips/minibomb").minibomb
local recov_chips = require("../../../chips/recov")
local repair = require("../../../chips/repair").repair
local spreader = require("../../../chips/spreader").spreader
local sword_chips = require("../../../chips/sword")

local chips = {}

-- The weapon `id` that loads `chip` (0x0800D406) and starts its use, charging for `ticks` (its row).
local function fires(id: string, chip: Chip, ticks: { number }): Weapon
    return new.weapon {
        id = id,
        charge_ticks = ticks,
        setup = function(navi: Object): Chip
            navi:load_chip_attack(chip)
            return chip
        end,
    }
end

-- Rows 0x29 to 0x5F of the charge table: 60 ticks at every Charge but Bubbler's Charge 5 (row 0x2D's first entry).
local SIXTY: { number } = { 60, 60, 60, 60, 60, 60 }

chips.cannon = fires("megaman/cannon", cannon, SIXTY) -- 0x29
chips.minibomb = fires("megaman/minibomb", minibomb, SIXTY) -- 0x2A
chips.heatshot = fires("megaman/heatshot", heatshot, SIXTY) -- 0x2B
chips.bubbler = fires("megaman/bubbler", bubbler, { 60, 60, 60, 60, 60, 120 }) -- 0x2C
chips.sword = fires("megaman/sword", sword_chips.sword, SIXTY) -- 0x2E
chips.spreader = fires("megaman/spreader", spreader, SIXTY) -- 0x2F
chips.crakout = fires("megaman/crakout", crakout_chips.crakout, SIXTY) -- 0x36
chips.dublcrak = fires("megaman/dublcrak", crakout_chips.dublcrak, SIXTY) -- 0x3C
chips.wideswrd = fires("megaman/wideswrd", sword_chips.wideswrd, SIXTY) -- 0x3E
chips.recov10 = fires("megaman/recov10", recov_chips.recov10, SIXTY) -- 0x40
chips.lance = fires("megaman/lance", lance, SIXTY) -- 0x41
chips.tripcrak = fires("megaman/tripcrak", crakout_chips.tripcrak, SIXTY) -- 0x48
chips.longswrd = fires("megaman/longswrd", sword_chips.longswrd, SIXTY) -- 0x49
chips.fullcust = fires("megaman/fullcust", fullcust, { 90, 90, 90, 90, 90, 180 }) -- 0x4B
chips.wideblde = fires("megaman/wideblde", sword_chips.wideblde, SIXTY) -- 0x52
chips.longblde = fires("megaman/longblde", sword_chips.longblde, SIXTY) -- 0x53
chips.repair = fires("megaman/repair", repair, SIXTY) -- 0x5E
chips.airshot = fires("megaman/airshot", airshot, SIXTY) -- 0x5F

return chips
