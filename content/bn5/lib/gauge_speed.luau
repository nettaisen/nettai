--!strict
-- SloGauge's and FstGauge's dimming controller (effect object #0x1C, BN6's code: @exelib/gauge-speed/controller),
-- BN5's kind; the warning's sound BN6's number.

local common = require("@exelib/gauge-speed/controller")

return common.make { id = "bn5:gauge-speed", warning = asset.sound("bn5:err-select-91") }
