--!strict
-- P.Driver (EXE4 chip 0x159): the Program Advance of GunSol1, GunSol2 and GunSol3 in that order (the recipe table,
-- the pointer list at 0x080212B0): a pile ahead of the user and another at the far end of the row, turned around,
-- each beaming ten times at the first of the other side's bodies it sees down the row. The dimming chips' variant
-- 0x3E (0x080E9CD4): the shared pile driver (@exelib/piledrvr: its controller, effect object #0x6F, 0x080E9C30; its
-- pile, attack object #0x99, 0x080E0318; its beam, #0x9A, 0x080E04A4: the code EXE5's PileDrvr kept) with EXE4's
-- look: the pile's sprite 0c-46 and sound 0x161, the beam's sprite 0c-4a and sound 0x162, its hits the erase spark's
-- (0x0C), piercing and breaking (row 0x16), hitting navis; its second pile beams at whom it sees whether or not the
-- first saw anyone (0x080E9DC6).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local beam = require("@exelib/piledrvr/beam")
local collision = require("../rules/collision")
local controller = require("@exelib/piledrvr/controller")
local gunsols = require("./gunsol")
local pile = require("@exelib/piledrvr/pile")
local sparks = require("../lib/sparks")

local BEAM = beam.make_kind {
    id = "pdriver/beam",
    sprite = asset.sprite("pile-beam"),
    sound = asset.sound("django-charge"),
    spark = sparks.erase,
    is = collision.piercing_break,
    hits = collision.hits_navis,
}

local PILE = pile.make_kind {
    id = "pdriver/pile",
    sprite = asset.sprite("pile-driver"),
    sound = asset.sound("pile-driver"),
    beam = BEAM,
}

local CONTROLLER = controller.make { id = "pdriver/controller", pile = PILE, second_alone = true }

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 130,
    lockout = 0,
    flags = { "dimming" },
    program_advances = { { order = 5, sequence = { gunsols.gunsol1, gunsols.gunsol2, gunsols.gunsol3 } } },
    dimming = CONTROLLER.hook,
}

return { pdriver = chip }
