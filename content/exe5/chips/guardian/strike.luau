--!strict
-- EXE5's Guardian strike-back dimming (effect object #0x53, 0x080E6890): the shared one (@exelib/guardian/strike, EXE6's
-- code), its telop naming EXE5's Punisher (chip 0x175, ../punisher/chip).

local common = require("@exelib/guardian/strike")
local punisher = require("../punisher")

return common.make { id = "guardian/strike", punisher = punisher }
