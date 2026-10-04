--!strict
-- BN5's panel chips' dimming controller (effect object #0x20, BN6's code: @exelib/panel-chips/controller), BN5's
-- kind with BN5's panel changer (objects/panel-changer).

local common = require("@exelib/panel-chips/controller")
local panel_changer = require("../objects/panel-changer/panel_changer")

local panel_chips = {}

local made = common.make { id = "panel-chips/controller", changer = panel_changer.kind }
panel_chips.kind = made.kind
panel_chips.hook = made.hook

-- The conditions most changes use.
panel_chips.OWN_AREA = common.OWN_AREA
panel_chips.OTHER_AREA = common.OTHER_AREA
panel_chips.ANY = common.ANY

return panel_chips
