--!strict
-- The weapon routines that fire a chip, which the patch cards name for MegaMan's B button, charged shot and B+Left
-- special (NaviStats +0x09, +0x0A, +0x0C: the table at 0x0800CA7C): each loads the chip as the attack (0x0800D406,
-- EXE5's 0x0800FE78: the chip's record's damage and hit parameter, lockout, element, variant and parameters; no extra)
-- and starts the chip's use. Each weapon is its routine (`megaman/<chip>`), its charge time the charge table's row
-- (0x080180B8, by the routine) by Charge 0 to 4, Charge 5 reading on into the next row's first entry.
--
-- The rest wait on their chips (docs/design/exe4-map.md §18 item 57).

local airshot = require("../../../chips/airshot").airshot
local ball = require("../../../chips/ball").ball
local breath_chips = require("../../../chips/breath")
local bubbler = require("../../../chips/bubbler").bubbler
local cannon = require("../../../chips/cannon").cannon
local crakout_chips = require("../../../chips/crakout")
local energbom_chips = require("../../../chips/energbom")
local fullcust = require("../../../chips/fullcust").fullcust
local guard_chips = require("../../../chips/guard")
local heatshot = require("../../../chips/heatshot").heatshot
local lance = require("../../../chips/lance").lance
local minibomb = require("../../../chips/minibomb").minibomb
local recov_chips = require("../../../chips/recov")
local repair = require("../../../chips/repair").repair
local spreader = require("../../../chips/spreader").spreader
local sword_chips = require("../../../chips/sword")
local thunder_chips = require("../../../chips/thunder")

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

-- Rows 0x29 to 0x62 of the charge table: 60 ticks at every Charge, but the Thunders' 120, 150 and 180, FullCust's 90,
-- and the Charge 5 of Bubbler, EnergBom, FullCust, ElecShok and Guard1 (rows 0x2D's, 0x46's, 0x4C's, 0x61's and 0x63's
-- first entries).
local SIXTY: { number } = { 60, 60, 60, 60, 60, 60 }

chips.cannon = fires("megaman/cannon", cannon, SIXTY) -- 0x29
chips.minibomb = fires("megaman/minibomb", minibomb, SIXTY) -- 0x2A
chips.heatshot = fires("megaman/heatshot", heatshot, SIXTY) -- 0x2B
chips.bubbler = fires("megaman/bubbler", bubbler, { 60, 60, 60, 60, 60, 120 }) -- 0x2C
chips.thunder1 = fires("megaman/thunder1", thunder_chips.thunder1, { 120, 120, 120, 120, 120, 60 }) -- 0x2D
chips.sword = fires("megaman/sword", sword_chips.sword, SIXTY) -- 0x2E
chips.spreader = fires("megaman/spreader", spreader, SIXTY) -- 0x2F
chips.crakout = fires("megaman/crakout", crakout_chips.crakout, SIXTY) -- 0x36
chips.thunder2 = fires("megaman/thunder2", thunder_chips.thunder2, { 150, 150, 150, 150, 150, 60 }) -- 0x3A
chips.dublcrak = fires("megaman/dublcrak", crakout_chips.dublcrak, SIXTY) -- 0x3C
chips.wideswrd = fires("megaman/wideswrd", sword_chips.wideswrd, SIXTY) -- 0x3E
chips.recov10 = fires("megaman/recov10", recov_chips.recov10, SIXTY) -- 0x40
chips.lance = fires("megaman/lance", lance, SIXTY) -- 0x41
chips.energbom = fires("megaman/energbom", energbom_chips.energbom, { 60, 60, 60, 60, 60, 180 }) -- 0x45
chips.thunder3 = fires("megaman/thunder3", thunder_chips.thunder3, { 180, 180, 180, 180, 180, 60 }) -- 0x46
chips.tripcrak = fires("megaman/tripcrak", crakout_chips.tripcrak, SIXTY) -- 0x48
chips.longswrd = fires("megaman/longswrd", sword_chips.longswrd, SIXTY) -- 0x49
chips.fullcust = fires("megaman/fullcust", fullcust, { 90, 90, 90, 90, 90, 180 }) -- 0x4B
chips.megenbom = fires("megaman/megenbom", energbom_chips.megenbom, SIXTY) -- 0x4F
chips.ball = fires("megaman/ball", ball, SIXTY) -- 0x50
chips.wideblde = fires("megaman/wideblde", sword_chips.wideblde, SIXTY) -- 0x52
chips.longblde = fires("megaman/longblde", sword_chips.longblde, SIXTY) -- 0x53
chips.blizzard = fires("megaman/blizzard", breath_chips.blizzard, SIXTY) -- 0x5B
chips.heatbrth = fires("megaman/heatbrth", breath_chips.heatbrth, SIXTY) -- 0x5C
chips.woodpwdr = fires("megaman/woodpwdr", breath_chips.woodpwdr, SIXTY) -- 0x5D
chips.repair = fires("megaman/repair", repair, SIXTY) -- 0x5E
chips.airshot = fires("megaman/airshot", airshot, SIXTY) -- 0x5F
chips.elecshok = fires("megaman/elecshok", breath_chips.elecshok, { 60, 60, 60, 60, 60, 80 }) -- 0x60
chips.guard1 = fires("megaman/guard1", guard_chips.guard1, { 60, 60, 60, 60, 60, 20 }) -- 0x62

return chips
