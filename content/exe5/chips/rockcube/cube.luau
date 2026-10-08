--!strict
-- EXE5's RockCube controller (effect object #0x37, 0x080E4664): the shared one (@exelib/rockcube/cube, EXE6's code)
-- with EXE5's rock (objects/rock: the cube, rising) and place sound. Its position is the dimming handler's registers:
-- the user's row, the element and the hook's own address (0x080E46E6, the handler's `bx r3`, Thumb).
-- (The address is Team ProtoMan US's ROM's; compat/games.toml has the other ROMs'.) Each chip's `dimming` hook places
-- its own rock (`cube.places { variant = ... }`, the record's first parameter's): RockCube's cube, WhiCapsl's ice.

local common = require("@exelib/rockcube/cube")
local rock = require("../../objects/rock")

local HOOK_ADDRESS = 0x080E46E7

local made = common.make {
    id = "rockcube/cube",
    spawn_rock = rock.spawn,
    place_sound = asset.sound("place"),
    position = function(user: Object, spec: DimmingChipSpec): Vec3
        return Vec3.new(user.panel_y, spec.element, HOOK_ADDRESS)
    end,
}

-- 0x080E46E6: a chip's `dimming` hook, its rock in front of the user.
return { places = made.places }
