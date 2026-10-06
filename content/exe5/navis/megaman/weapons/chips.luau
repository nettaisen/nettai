--!strict
-- The weapon routines that fire a chip, which the patch cards name for MegaMan's B button, charged shot and B+Back
-- special (docs/design/exe5-map.md §15.14): each loads a chip as the attack (0x0800FE78, EXE6's `loc_80126EA`'s place:
-- the chip record's damage, hit parameter, lockout, element, subtype and parameters; no bonus; not charged) and
-- starts the chip's use. Each weapon is its routine (`megaman/<chip>`), its charge time the charge table's row
-- (0x0801CA6C) by Charge 0 to 5, Charge 5 reading on into the next row's first entry.
--
-- Three do more than load: MettGuard's and CrsShld's B+Back (0x6A, 0x6B) wait 40 ticks before the next chip (their
-- lockout, over the record's), and Ccann's charged shot (0x5F) takes TankCan1's shell's cracking away (its second
-- parameter 0: the edge blast leaves the panels whole), the TankCans' action with that shell.

local airhoc = require("../../../chips/airhoc").airhoc
local blkbomb = require("../../../chips/blkbomb").blkbomb
local boomer = require("../../../chips/boomer").boomer
local cannball = require("../../../chips/cannball").cannball
local circgun = require("../../../chips/circgun").circgun
local crakout_chips = require("../../../chips/crakout")
local crsshld_chips = require("../../../chips/crsshld")
local drilarm_chips = require("../../../chips/drilarm")
local drksword = require("../../../chips/drksword").drksword
local guard_chips = require("../../../chips/guard")
local katana_chips = require("../../../chips/katana")
local longswrd = require("../../../chips/longswrd").longswrd
local minibomb = require("../../../chips/minibomb").minibomb
local moonbld_chips = require("../../../chips/moonbld")
local poltrgst = require("../../../chips/poltrgst").poltrgst
local pulsar_chips = require("../../../chips/pulsar")
local seaseed = require("../../../chips/seaseed").seaseed
local sword = require("../../../chips/sword").sword
local tankcan = require("../../../chips/tankcan/tankcan")
local tankcan_chips = require("../../../chips/tankcan")
local thunder = require("../../../chips/thunder").thunder
local tornado_chips = require("../../../chips/tornado")
local wideblde = require("../../../chips/wideblde").wideblde
local widesht_chips = require("../../../chips/widesht")
local wideswrd = require("../../../chips/wideswrd").wideswrd
local yoyo = require("../../../chips/yoyo").yoyo

local chips = {}

-- The weapon `id` that loads `chip` (0x0800FE78) and starts its use, charging for `ticks` (its row).
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

-- A guard's B+Back: the chip loaded, and 40 ticks before the next chip; the chip's own action (action 0x1F, EXE5's
-- guard: the routine names it, the chip's record does too).
local GUARD_LOCKOUT = 40
local function guards(id: string, chip: Chip, ticks: { number }): Weapon
    return new.weapon {
        id = id,
        charge_ticks = ticks,
        setup = function(navi: Object): Chip
            navi:load_chip_attack(chip)
            navi.attack_lockout = GUARD_LOCKOUT
            return chip
        end,
    }
end

-- ---- Charged shots ----------------------------------------------------------------------------------------------------

chips.longswrd = fires("megaman/longswrd", longswrd, { 80, 75, 70, 65, 60, 120 }) -- 0x48, B↓LongS
chips.thunder = fires("megaman/thunder", thunder, { 120, 115, 110, 105, 100, 60 }) -- 0x49, B↓Thundr
chips.yoyo = fires("megaman/yoyo", yoyo, { 80, 75, 70, 65, 60, 70 }) -- 0x4B, B↓YoYo
chips.katana = fires("megaman/katana1", katana_chips.katana1, { 90, 85, 80, 75, 70, 70 }) -- 0x4D, B↓Katana
chips.pulsar = fires("megaman/pulsar1", pulsar_chips.pulsar1, { 110, 105, 100, 95, 90, 130 }) -- 0x50, B↓Pulsar
chips.poltrgst = fires("megaman/poltrgst", poltrgst, { 130, 125, 120, 115, 110, 80 }) -- 0x51, B↓Poltr
chips.wideswrd = fires("megaman/wideswrd", wideswrd, { 80, 75, 70, 65, 60, 70 }) -- 0x52, B↓WideS
-- (0x53: B↓MiniBm and B-Bomb.)
chips.minibomb = fires("megaman/minibomb", minibomb, { 70, 65, 60, 55, 50, 100 })
chips.cannball = fires("megaman/cannball", cannball, { 100, 95, 90, 85, 80, 120 }) -- 0x54, B↓CanBal
chips.tornado = fires("megaman/tornado", tornado_chips.tornado, { 120, 115, 110, 105, 100, 60 }) -- 0x55, B↓Trnado
chips.moonbld = fires("megaman/moonbld1", moonbld_chips.moonbld1, { 100, 95, 90, 85, 80, 140 }) -- 0x58, B↓MonBld
chips.airhoc = fires("megaman/airhoc", airhoc, { 140, 135, 130, 125, 120, 150 }) -- 0x59, B↓AirHoc
chips.circgun = fires("megaman/circgun", circgun, { 150, 145, 140, 135, 130, 120 }) -- 0x5A, B↓CircG
chips.drilarm = fires("megaman/drilarm1", drilarm_chips.drilarm1, { 120, 115, 110, 105, 100, 100 }) -- 0x5B, B↓DrlArm
chips.wideblde = fires("megaman/wideblde", wideblde, { 100, 95, 90, 85, 80, 160 }) -- 0x5C, B↓WideB
chips.blkbomb = fires("megaman/blkbomb", blkbomb, { 160, 155, 150, 145, 140, 100 }) -- 0x5D, B↓BlkBom
chips.seaseed = fires("megaman/seaseed", seaseed, { 100, 95, 90, 85, 80, 100 }) -- 0x5E, B↓SeaSed
chips.drksword = fires("megaman/drksword", drksword, { 180, 175, 170, 165, 160, 40 }) -- 0x60, B↓DarkS
chips.boomer = fires("megaman/boomer", boomer, { 120, 115, 110, 105, 100, 120 }) -- 0x66, B↓Boomer
chips.widesht = fires("megaman/widesht1", widesht_chips.widesht1, { 120, 115, 110, 105, 100, 110 }) -- 0x67, B↓Wshot

-- 0x5F, B↓Ccann: TankCan1, its shell not cracking (the parameter's second byte 0).
local CCANN = tankcan.make_action({ hit_mod = 0x4B, cracks = false, shift = 0 }, "megaman/tankcan1/action")
chips.tankcan = new.weapon {
    id = "megaman/tankcan1",
    charge_ticks = { 100, 95, 90, 85, 80, 180 },
    setup = function(navi: Object): Action
        navi:load_chip_attack(tankcan_chips.tankcan1)
        return CCANN
    end,
}

-- ---- B buttons -----------------------------------------------------------------------------------------------------

chips.sword = fires("megaman/sword", sword, { 80, 75, 70, 65, 60, 60 }) -- 0x64, B-Sword
chips.crakout = fires("megaman/crakout", crakout_chips.crakout, { 50, 45, 40, 35, 30, 100 }) -- 0x57, B-Crack

-- ---- B+Back specials -----------------------------------------------------------------------------------------------

chips.mettguard = guards("megaman/guard1", guard_chips.guard1, { 60, 55, 50, 45, 40, 60 }) -- 0x6A, B+←Guard
chips.crsshld = guards("megaman/crsshld1", crsshld_chips.crsshld1, { 60, 55, 50, 45, 40, 2052 }) -- 0x6B, B+←CurS

return chips
