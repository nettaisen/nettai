--!strict
-- Magnum (EXE4 chip 0x05E): a dimming chip: the user's stand-in moves a sight across the enemy's columns until A or 180
-- ticks, then shoots the sight's column, breaking its panels. The dimming chips' action 0x0C, its variant 0x16
-- (0x080220E0's): EXE5's controller and gunner (@exelib/magnum), EXE5's grown from EXE4's. Its controller (effect object
-- #0x30, 0x080E57F8; its spawner 0x080E5878 the usual one) sends out EXE4's gunner (attack object #0x50, 0x080D756C;
-- spawned by 0x080D798E): drawn as the user and wearing nothing (0x080D7590), its sounds 0x128 (a step of the sight),
-- 0xC0 (the shot), 0xBC (the blast) and 0x95 (a panel broken), firing in animation 0x10, the sight of effect row 0x49
-- (10-22), and the sight stepping to the next column with an enemy panel or else from the field's start (0x080D776E).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common_controller = require("@exelib/magnum/controller")
local common_gunner = require("@exelib/magnum/gunner")
local sparks = require("../lib/sparks")

local GUNNER = common_gunner.make {
    id = "magnum/gunner",
    pool = "attack",
    -- (0x080D7590 puts nothing of its user's on it.)
    wear = function(_me: Object, _user: Object) end,
    step_sound = asset.sound("scope-move"),
    shot = asset.sound("hockey-puck"),
    blast = asset.sound("heat-burst"),
    crack = asset.sound("panel-crack"),
    fire_anim = 0x10,
    sight = new.effect { sprite = asset.sprite("search-scope"), anim = 0, palette = 0 },
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    spark = sparks.heavy,
    is = collision.thrown,
    hits = collision.hits_navis,
    step = common_gunner.next_or_wrap,
}

local CONTROLLER = common_controller.make { id = "magnum/controller", gunner = GUNNER.spawn }

local chip: Chip = {
    codes = { "H", "Q", "V" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 130,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook,
    auto_battle = { class = "into_reach", range = "ahead_then_column" },
}

return { magnum = chip }
