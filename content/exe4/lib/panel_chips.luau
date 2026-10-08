--!strict
-- EXE4's panel chips' dimming controller (the dimming chips' variant 0x0B: effect object #0x20, 0x080E4584; its effect
-- 0x080E45C8): EXE6's and EXE5's code (@exelib/panel_chips/controller) with EXE4's panel changer
-- (../objects/panel_changer), its spawn (0x080E45F6) copying the user's side, not its facing. A chip's change is a row
-- of the changer's table (the chip's first parameter), which each chip states: Hole, PnlRetrn, HolyPanl, Snctuary and
-- DrkLine.

local common = require("@exelib/panel_chips/controller")
local panel_changer = require("../objects/panel_changer")

local panel_chips = {}

local made = common.make { id = "panel-chips/controller", changer = panel_changer.kind, side_only = true }
panel_chips.kind = made.kind
panel_chips.hook = made.hook

return panel_chips
