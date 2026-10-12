--!strict
-- BodyGrd (EXE4 chip 0x15C): the Program Advance of AntiDmg, AntiNavi and Muramasa in that order (the recipe table,
-- the pointer list at 0x080212B0): an unseen striker drops ten shurikens on the enemy navi, one every 20 ticks. The
-- dimming chips' variant 0x3D (0x080E9ABC): the shared controller (@exelib/bodygrd/controller, effect object #0x6D,
-- 0x080E9A50), done at once (the chip's second parameter, 0), sends out the shared striker (@exelib/bodygrd/striker,
-- effect object #0x6E, 0x080E9AE0: EXE6's code) with EXE4's look: no NameID, the throw's sound 0xAB, and the diving
-- shuriken (attack object #0x5C, 0x080D8EFC: @exelib/bodygrd/shuriken, EXE6's code, with EXE4's look as AntiDmg's
-- counter's has it, chips/anti/counter: 0c-2f, the crack's sound, effect row 2 as it shows, a thrown slash hitting
-- navis; no more at its dive's start).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local antis = require("./anti")
local collision = require("../rules/collision")
local controller = require("@exelib/bodygrd/controller")
local recipe_muramasa = require("./muramasa").muramasa
local shuriken = require("@exelib/bodygrd/shuriken")
local striker = require("@exelib/bodygrd/striker")

local SHURIKEN = shuriken.make_kind {
    id = "bodygrd/shuriken",
    sprite = asset.sprite("shuriken"),
    land_sound = asset.sound("panel-crack"),
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    self_type = collision.thrown_slash,
    target = collision.hits_navis,
}

local STRIKER = striker.make_kind {
    id = "bodygrd/striker",
    -- EXE4 writes no NameID. What the poltergeist and ColdMan leave alone they tell by kind (0x0800B3E8: effect object
    -- #0x6E among them): the identity says so.
    identity = new.identity { class = "field_object", version = 0, ai_index = 0, throwable = false },
    shuriken = SHURIKEN,
    throw_sound = asset.sound("boomerang"),
}

local CONTROLLER = controller.make("bodygrd/controller", STRIKER)

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = { { order = 7, sequence = { antis.antidmg, antis.antinavi, recipe_muramasa } } },
    dimming = CONTROLLER.hook(0),
}

return { bodygrd = chip }
