--!strict
-- SloGauge's and FstGauge's dimming controller (effect object #0x1C, EXE6's code: @exelib/gauge_speed/controller),
-- EXE5's kind; the warning's sound EXE6's number.

local common = require("@exelib/gauge_speed/controller")

return common.make { id = "gauge-speed", warning = asset.sound("err-select-91") }
