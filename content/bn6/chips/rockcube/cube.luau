--!strict
-- RockCube's and IceCube's dimming controller (the original's effect object #0x37, `sub_80E4634`): the shared one
-- (@exelib/rockcube/cube) with BN6's rock (./rock) and place sound.

local common = require("@exelib/rockcube/cube")
local rock = require("./rock")

local cube = {}

local made = common.make {
    id = "rockcube/cube",
    spawn_rock = rock.spawn,
    place_sound = asset.sound("place"),
}

-- A chip's `dimming` hook (`sub_80E46B6`, `off_802CCB4[6]`): the controller that places a rock of `placed`.
cube.places = made.places

return cube
