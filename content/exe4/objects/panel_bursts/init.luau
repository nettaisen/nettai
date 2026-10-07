--!strict
-- EXE4's panel bursts (effect object 0x24, 0x080E4A0A: EXE6's code, @exelib/panel_bursts), EXE4's kind. Its spawner
-- (0x080E4A9C) takes a whole-field region alone (the panel filters of 0x08012618 by its argument's low bits): the
-- Anti chips' counterattack bursts over the other side's area with it (lib/traps/anti_trap).

local common = require("@exelib/panel_bursts")

local panel_bursts = {}

panel_bursts.kind = common.make_kind("panel-bursts")

return panel_bursts
