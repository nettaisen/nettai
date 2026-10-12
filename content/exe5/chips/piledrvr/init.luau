--!strict
-- PileDrvr (EXE5 chip 0x15d): a Program Advance (GunDelS3, GunDelS3 and DjangoSP or DjangoDS) whose cut-in sets down a
-- pile ahead of the user and another at the far end of the row, turned around, each beaming ten times at the first of
-- the other side's bodies it sees down the row: the shared pile driver (@exelib/piledrvr: its controller, EXE5's
-- effect object #0x6F, 0x080E8800; its pile, attack object #0x99, 0x080D9C1C; its beam, #0x9A, 0x080D9DA0; EXE6's
-- code that no EXE6 chip uses) with EXE5's look: the pile's sprite 0C-46 and sound, the beam's sprite 0C-4A and
-- sound, its hits the erase spark's, piercing and breaking, hitting navis; a second pile beams at no one when the
-- first saw no one (0x080E89AC).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local beam = require("@exelib/piledrvr/beam")
local collision = require("../rules/collision")
local controller = require("@exelib/piledrvr/controller")
local pile = require("@exelib/piledrvr/pile")
local recipe_django_ds = require("./django")["django-ds"]
local recipe_django_sp = require("./django")["django-sp"]
local recipe_gundels3 = require("./gundels").gundels3
local sparks = require("../lib/sparks")

local BEAM = beam.make_kind {
    id = "piledrvr/beam",
    sprite = asset.sprite("pile-beam"),
    sound = asset.sound("django-charge"),
    spark = sparks.erase,
    is = collision.piercing_break,
    hits = collision.hits_navis,
}

local PILE = pile.make_kind {
    id = "piledrvr/pile",
    sprite = asset.sprite("pile-driver"),
    sound = asset.sound("pile-driver"),
    beam = BEAM,
}

local CONTROLLER = controller.make { id = "piledrvr/controller", pile = PILE, second_alone = false }

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 30,
    hit_param = 130,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 32, sequence = { recipe_gundels3, recipe_gundels3, recipe_django_sp } },
        { order = 33, sequence = { recipe_gundels3, recipe_gundels3, recipe_django_ds } },
    },
    dimming = CONTROLLER.hook,
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { piledrvr = chip }
