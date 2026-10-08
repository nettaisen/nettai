--!strict
-- RockCube (EXE4 chip 0x063): the dimming chips' variant 0x1B, a rock cube rising on the panel in front of the user:
-- its controller (effect object #0x37, 0x080E60AC: @exelib/rockcube/cube, EXE6's and EXE5's code) in EXE4's look:
-- EXE4's rock (objects/rock, attack object #0x59), the place sound (0xAA); its spawn (0x080E612C) takes the user's side
-- alone, and its effect places the rock by the side's way (0x080E60F8). The record's parameter word (1) is the rock's
-- row of 0x080D89C0, its class and entrance: row 1, the cube of 200 HP, as the weapon bug's rock cube's
-- (navis/megaman/weapons/bug_shots); class 0, rising.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local common = require("@exelib/rockcube/cube")
local rock = require("../objects/rock")

-- Row 1 of 0x080D89C0: the cube standing in animation 1, 200 HP (100 in twos), its debris in palette 0, the panel
-- crack's sound as it breaks.
local CUBE = rock.variant {
    anim = 1,
    hp = 200,
    element = "null",
    debris_palette = 0,
    break_sound = asset.sound("panel-crack"),
    identity = nil,
}

local CONTROLLER = common.make {
    id = "rockcube/cube",
    spawn_rock = rock.spawn,
    place_sound = asset.sound("place"),
    side_only = true,
}

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 200,
    hit_param = 50,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = CONTROLLER.places { variant = CUBE, class = 0, entrance = "rise" },
}

return { rockcube = chip }
