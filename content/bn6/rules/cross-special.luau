--!strict
-- The Cross special's chips (`sub_802D5A8`, the chip gate battle's
-- berserk controller): a row by the hundreds of the navi's base max HP
-- (the first up to 199, the last from 1000), and a chip of it at random.
-- LifeSrd, in the last two rows, strikes with VarSwrd's damage
-- (`damage_of`).

local airhocky = require("../chips/airhocky")
local aquaswrd = require("../chips/aquaswrd")
local bambswrd = require("../chips/bambswrd")
local cannons = require("../chips/cannon")
local dolthdr = require("../chips/dolthdr")
local elecswrd = require("../chips/elecswrd")
local energbom = require("../chips/energbom")
local firebrn = require("../chips/firebrn")
local fireswrd = require("../chips/fireswrd")
local gundels = require("../chips/gundels")
local lifesrd = require("../chips/lifesrd")
local longswrd = require("../chips/longswrd")
local minibomb = require("../chips/minibomb")
local moonbld = require("../chips/moonbld")
local rlnglog = require("../chips/rlnglog")
local tankcan = require("../chips/tankcan")
local tornado = require("../chips/tornado")
local trnarrw = require("../chips/trnarrw")
local varswrd = require("../chips/varswrd")
local vulcan = require("../chips/vulcan")
local wideswrd = require("../chips/wideswrd")

-- (The beast system's data, which its Cross special reads: a record of
-- type "cross-special", not a rule section the engine reads.)
return define.record("cross-special", {
    id = "cross-special",
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
        { tankcan[3], airhocky, { chip = lifesrd, damage_of = varswrd } },
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
