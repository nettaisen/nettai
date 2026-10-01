--!strict
-- The Cross special's chips (`sub_802D5A8`, the battle flag 0x40 mode's
-- berserk controller): a row by the hundreds of the navi's base max HP
-- (the first up to 199, the last from 1000), and a chip of it at random.
-- The last row's LifeSrd strikes with VarSwrd's damage (`damage_of`).

local airhocky = require("../chips/airhocky/chip")
local aquaswrd = require("../chips/aquaswrd/chip")
local bambswrd = require("../chips/bambswrd/chip")
local cannons = require("../chips/cannon/chips")
local dolthdr = require("../chips/dolthdr/chips")
local elecswrd = require("../chips/elecswrd/chip")
local energbom = require("../chips/energbom/chips")
local firebrn = require("../chips/firebrn/chips")
local fireswrd = require("../chips/fireswrd/chip")
local gundels = require("../chips/gundels/chips")
local lifesrd = require("../chips/lifesrd/chip")
local longswrd = require("../chips/longswrd/chip")
local minibomb = require("../chips/minibomb/chip")
local moonbld = require("../chips/moonbld/chip")
local rlnglog = require("../chips/rlnglog/chips")
local tankcan = require("../chips/tankcan/chips")
local tornado = require("../chips/tornado/chips")
local trnarrw = require("../chips/trnarrw/chips")
local varswrd = require("../chips/varswrd/chip")
local vulcan = require("../chips/vulcan/chips")
local wideswrd = require("../chips/wideswrd/chip")

return define.rules("cross-special", {
    rows = {
        { cannons.cannon, minibomb, wideswrd },
        -- EnergBom.
        { cannons.cannon, energbom[1], wideswrd },
        -- MegEnBom.
        { cannons.hicannon, energbom[2], longswrd },
        { tankcan[1], firebrn.firebrn2, fireswrd },
        { cannons.m_cannon, trnarrw[2], aquaswrd },
        { tankcan[2], dolthdr[2], elecswrd },
        -- GunDelS2.
        { gundels[2], rlnglog[2], bambswrd },
        { tankcan[3], tornado[1], moonbld },
        { tankcan[3], airhocky, lifesrd },
        -- (SuprVulc.)
        {
            firebrn.firebrn3,
            trnarrw[3],
            dolthdr[3],
            rlnglog[3],
            vulcan[4],
            { chip = lifesrd, damage_of = varswrd },
        },
    },
})
