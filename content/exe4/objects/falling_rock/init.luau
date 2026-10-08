--!strict
-- EXE4's falling rock (attack object #0x1D, 0x080D08BC: @exelib/rock/falling, EXE6's #0x1D's code but for its
-- constants) and the chips it breaks into (effect object #9, 0x080E2914: @exelib/rock/falling_chip): a rock 160
-- pixels over its panel, falling 1/16 pixel a tick faster each tick; an attack on navis (hit modifier 3, the plain
-- spark); a puff into a hole (effect row 0x15); its chips on sheet 10-0E with their crash.
--
-- Each user makes its kind (`falling_rock.make`): GutsMan's quake (chips/gutsman), actor #0x29 (0x080BFCC8).

local collision = require("../rules/collision")
local falling = require("@exelib/rock/falling")
local falling_chip = require("@exelib/rock/falling_chip")
local rock = require("../lib/rock")
local sparks = require("../lib/sparks")

local falling_rock = {}

export type FallingRock = {
    kind: Kind,
    chips: Kind,
    spawn: (owner: Object, x: number, y: number, element: number, damage: number) -> Object?,
}

-- The rock's sheet (10-0E).
local SPRITE = asset.sprite("falling-rock")

-- A user's falling rock: its kind (`id`) and its chips' (`id`'s "-chip").
function falling_rock.make(id: string): FallingRock
    local chips = falling_chip.make_kind(id .. "-chip", SPRITE)
    local kind = falling.make_kind {
        id = id,
        sprite = SPRITE,
        height = 0xA0_0000,
        gravity = 0x1000,
        is = collision.attack,
        hits = collision.hits_navis,
        hit_mod = 3,
        spark = sparks.plain,
        puff = rock.effects.puff,
        chips = chips,
        shatter_sound = asset.sound("falling-rock"),
    }

    -- 0x080D0A40: a rock on (x, y) of `owner`'s side, with `element` and the damage word. (Its position is the
    -- spawner's registers: the row, the element, and what the caller left in r3, which its init overwrites.)
    local function spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, element, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        return o
    end

    return { kind = kind, chips = chips.kind, spawn = spawn }
end

return falling_rock
