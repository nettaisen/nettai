--!strict
-- A chip of broken rock (the original's effect object #9, `sub_80E0FA0`: @exelib/rock/falling_chip, EXE6's kind):
-- spawned with a sideways and an upward speed (`sub_80E1084`), it flies and falls; below the ground it blinks for 10
-- ticks and is gone. A falling rock (./init) breaks into two, as GroundMan's rocks do (chips/grndman/rock).

local falling_chip = require("@exelib/rock/falling_chip")

return falling_chip.make_kind("falling-rock/chip", asset.sprite("falling-rock"))
