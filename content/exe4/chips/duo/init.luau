--!strict
-- Duo (EXE4 chip 0x137): an e-Reader card's chip. Its record and its behavior are in every ROM; its name, its
-- description, its picture and the picture's palette are the save's card data (the record points at RAM: the
-- extractor's exe4/cards.rs), the palette here as the cards give it, as EXE5's e-Reader chips' and EXE6's gift chips'
-- are.
--
-- The dimming chips' action 0x0C, its variant 0x47 (0x080EA876): a navi chip. Its controller (effect object #0x79,
-- 0x080EA7D8) runs the navi chips' short course (lib/navi_chips: the user warps out, 11 ticks, the navi, 21 ticks
-- after he left, the undim with the user warping back in). As its navi's turn comes every field object's HP goes to 0
-- (0x080EA898: the registry's eight slots, either side's and the stage's), and then Duo comes (./navi: he takes his
-- side's back two columns and rains 17 fists on and around the enemies), unless an enemy stands on one of those six
-- panels (0x080EA8B4: a search of them that draws from the RNG when it finds one): then no one comes, and the course
-- goes on.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs, its palette from the
-- card data in Tango's save templates.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local CONTROLLER = navi_chips.controller {
    id = "duo/controller",
    name = "Duo",
    spawn = function(c: Object, _user: Object, damage: number): Object?
        for _, o in field.all_objects() do
            o.hp = 0
        end
        -- His side's back two columns, from the middle row of the last (column 1, or 6 for side 1: region 0x11).
        if panels.random_enemy_panel(c, c.alliance * 5 + 1, 2, regions.wide_long) then
            return nil
        end
        return navi.summon(c, damage)
    end,
    short = navi_chips.SHORT,
}

local chip: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 200,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    art_palette = { 0x0000, 0x1839, 0x1940, 0x3206, 0x299F, 0x0880, 0x1C63, 0x24C5, 0x3127, 0x3989, 0x41CB, 0x460D, 0x5271, 0x62F5, 0x6F79, 0x7BDD },
    dimming = CONTROLLER.hook,
    auto_battle = { class = "at_once" },
}

return { duo = chip }
