--!strict
-- BN5's RockCube controller (effect object #0x37, 0x080E4664): the shared one (@exelib/rockcube/cube, BN6's code)
-- with BN5's rock (objects/rock: the cube, rising) and place sound. Its position is the dimming handler's registers:
-- the user's row, the element and the hook's own address (0x080E46E6, the handler's `bx r3`, Thumb).

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

return {
    -- 0x080E46E6: the chip's `dimming` hook, a rock cube in front of the user (the record's first parameter, 1).
    hook = made.places { variant = rock.variants.cube },
}
