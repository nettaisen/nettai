--!strict
-- The Cross special's chips (`sub_802D5A8`, the chip gate battle's
-- berserk controller): a row by the hundreds of the navi's base max HP
-- (the first up to 199, the last from 1000), and a chip of it at random.
-- LifeSrd, in the last two rows, strikes with VarSwrd's damage
-- (`damage_of`).

local airhocky = require("../chips/airhocky")
local aquaswrd = require("../chips/aquaswrd")
local bambswrd = require("../chips/bambswrd")
local cannon_chips = require("../chips/cannon")
local dolthdr_chips = require("../chips/dolthdr")
local elecswrd = require("../chips/elecswrd")
local energbom_chips = require("../chips/energbom")
local firebrn_chips = require("../chips/firebrn")
local fireswrd = require("../chips/fireswrd")
local gundels_chips = require("../chips/gundels")
local lifesrd = require("../chips/lifesrd")
local longswrd = require("../chips/longswrd")
local minibomb = require("../chips/minibomb")
local moonbld = require("../chips/moonbld")
local rlnglog_chips = require("../chips/rlnglog")
local tankcan_chips = require("../chips/tankcan")
local tornado_chips = require("../chips/tornado")
local trnarrw_chips = require("../chips/trnarrw")
local varswrd = require("../chips/varswrd")
local vulcan_chips = require("../chips/vulcan")
local wideswrd = require("../chips/wideswrd")

-- (The beast system's data, which its Cross special reads: a record of
-- type "cross-special", not a rule section the engine reads.)
return define.record("cross-special", {
    id = "cross-special",
    rows = {
        { cannon_chips.cannon, minibomb, wideswrd },
        -- EnergBom.
        { cannon_chips.cannon, energbom_chips.energbom, wideswrd },
        -- MegEnBom.
        { cannon_chips.hicannon, energbom_chips.megenbom, longswrd },
        { tankcan_chips.tankcan1, firebrn_chips.firebrn2, fireswrd },
        { cannon_chips.m_cannon, trnarrw_chips.trnarrw2, aquaswrd },
        { tankcan_chips.tankcan2, dolthdr_chips.dolthdr2, elecswrd },
        -- GunDelS2.
        { gundels_chips.gundels2, rlnglog_chips.rlnglog2, bambswrd },
        { tankcan_chips.tankcan3, tornado_chips.tornado, moonbld },
        { tankcan_chips.tankcan3, airhocky, { chip = lifesrd, damage_of = varswrd } },
        -- (SuprVulc.)
        {
            firebrn_chips.firebrn3,
            trnarrw_chips.trnarrw3,
            dolthdr_chips.dolthdr3,
            rlnglog_chips.rlnglog3,
            vulcan_chips.suprvulc,
            { chip = lifesrd, damage_of = varswrd },
        },
    },
})
