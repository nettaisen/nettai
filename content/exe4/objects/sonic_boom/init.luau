--!strict
-- EXE4's sonic boom (attack object 0x58, 0x080D8878: EXE6's code, @exelib/swords/sonic_boom), EXE4's kind with EXE5's
-- look: the sword's slash (0c-14) on the ground, palette 9 going through, else by its element (0x080D8928), a slash
-- (row 7) hitting navis. AntiSwrd's counter throws it (chips/anti/sword).

local collision = require("../rules/collision")
local common = require("@exelib/swords/sonic_boom")

local sonic_boom = {}

sonic_boom.kind = common.make_kind("sonic-boom", {
    sprite = asset.sprite("sword-slash"),
    z = 0,
    piercing_palette = 9,
    palettes = { [0] = 0, 2, 5, 4, 8, 0, 0, 0 },
    element_mask = 0xFF,
    is = collision.slash,
    hits = collision.hits_navis,
})

return sonic_boom
