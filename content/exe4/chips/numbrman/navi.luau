--!strict
-- NumberMan, the navi of EXE4's NumberMan chips (EXE4's actor object #0x0F, 0x080B87AC), and each chip's controller
-- (effect object #0x0D, 0x080E2EC0: lib/navi_chips' long course, 0x080E2F08).
--
-- NumberMan is EXE5's (@exelib/numbrman/navi: the same code), with EXE4's look (his sprite, 08-08, in his chip's
-- palette; his sounds), his face and his die, EXE5's too (@exelib/numbrman/face and dice: actor #0x5C, 0x080CCC9C;
-- attack #5, 0x080CD9E8), with EXE4's differences:
--
-- - the face, with the fight on, follows him from its first update (0x080CCD0A);
-- - the die states its own 40 HP and hit modifier 3 and no status (0x080CDA44, 0x080CDA74), places itself on its panel
--   in its init (0x080CDA4A), has no markers and no face 9 of its own, and multiplies its damage's whole halfword by
--   its face (0x080CDC6E); its effects are EXE4's rows 0 (its explosion), 2 (off solid ground) and 0x15 (the battle's
--   end), its sounds EXE4's (its bounce TopMan's spin's);
-- - his spawner (0x080B8998) keeps his controller as his first related object, and his height is register garbage
--   his init overwrites.

local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi_chips")

local FACE = require("@exelib/numbrman/face").make {
    id = "numbrman/face",
    sprite = asset.sprite("numberman-face"),
    follows_in_fight = true,
}

local DICE = require("@exelib/numbrman/dice").make {
    id = "numbrman/dice",
    sprite = asset.sprite("dice"),
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    gone = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    blow_up = asset.sound("damage"),
    bounce = asset.sound("top-spin"),
    burst = asset.sound("heat-burst"),
    -- NumberMan's: an attack at navis (rows 4 and 5); the others' (the NumberMan boss's): their own body, which the
    -- other side or a blocker hits (rows 12 and 13).
    attack = collision.attack,
    hits_navis = collision.hits_navis,
    body = collision.own_object,
    hit_by = collision.hit_by_other_side_or_blockers,
    places_itself = true,
}

local NUMBERMAN = require("@exelib/numbrman/navi").make {
    id = "numbrman/navi",
    sprite = asset.sprite("numberman"),
    appear = asset.sound("appear"),
    throw = asset.sound("minibomb-throw"),
    face = FACE,
    dice = DICE,
    die = { hp = 40, status = nil, hit_mod = 3 },
}

-- 0x080B8998: NumberMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His
-- position is the spawner's registers: the row, the element and whatever the controller left in r3, all of which his
-- init overwrites.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(NUMBERMAN.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.related1 = controller
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    NUMBERMAN.summoned(o, controller, palette)
    return o
end

-- What each NumberMan chip states for its own controller: the controller's id (the original's effect object #0x0D for
-- all three: compat maps each to it) and his palette (0x0800B950 by the chip's level: 0, the SP's 2, the DS's 3).
export type ChipSpec = { id: string, palette: number }

-- A NumberMan chip's controller: the long course (the user warps out, NumberMan, 31 ticks, the user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "NumberMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        steps = navi_chips.LONG,
    }
end

return { controller = controller }
