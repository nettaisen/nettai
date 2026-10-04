--!strict
-- BN5's Guardian strike-back dimming (effect object #0x53, 0x080E6890): the shared one (@exelib/guardian/strike, BN6's
-- code), its telop naming BN5's Punisher (chip 0x175, ../punisher/chip).

local common = require("@exelib/guardian/strike")
local punisher = require("../punisher/chip")

return common.make { id = "bn5:guardian/strike", punisher = punisher }
