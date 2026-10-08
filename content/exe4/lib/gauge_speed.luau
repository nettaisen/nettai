--!strict
-- SloGauge's and FstGauge's dimming controller (effect object #0x1C, 0x080E3F0C: EXE6's code, @exelib/gauge_speed/
-- controller), EXE4's kind: its effect (0x080E3F50) sets the gauge's rate (0x10 or 0x40) and the user's side's slow or
-- fast timer (0x1E0 ticks; EXE4 has no own-gauges mode, so never the other side's), then warns over the gauge for 70
-- ticks with 0x79 (0x08008424 at (120, 12)), on both consoles (the user's alone only under event flag 0x1187, which no
-- netbattle sets). Its spawn (0x080E3FE4) takes the user's side alone.

local common = require("@exelib/gauge_speed/controller")

return common.make { id = "gauge-speed", warning = asset.sound("program-advance-part") }
