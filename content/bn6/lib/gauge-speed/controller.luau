--!strict
-- SloGauge's and FstGauge's dimming controller (the original's effect
-- object #0x1C, `sub_80E23A4`): the shared controller
-- (@common/gauge-speed/controller), BN6's kind and warning sound. The
-- chips' `dimming` hooks are `gauge_speed.slow` and `gauge_speed.fast`.

local common = require("@common/gauge-speed/controller")

return common.make { id = "bn6:gauge-speed", warning = asset.sound("bn6:err-select-91") }
