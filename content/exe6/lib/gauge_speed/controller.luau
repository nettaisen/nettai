--!strict
-- SloGauge's and FstGauge's dimming controller (the original's effect
-- object #0x1C, `sub_80E23A4`): the shared controller
-- (@exelib/gauge_speed/controller), EXE6's kind and warning sound. The
-- chips' `dimming` hooks are `gauge_speed.slow` and `gauge_speed.fast`.

local common = require("@exelib/gauge_speed/controller")

return common.make { id = "gauge-speed", warning = asset.sound("err-select-91") }
