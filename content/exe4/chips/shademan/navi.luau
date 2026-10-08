--!strict
-- ShadeMan, the navi of EXE4's ShadeMan chips (EXE4's actor object #0x19, 0x080BAD70), and each chip's controller
-- (effect object #0x15, 0x080E3888: lib/navi_chips' short course).
--
-- ShadeMan and his crush (attack #0x34, 0x080D3748) are EXE5's (@exelib/shademan: the same code), with EXE4's look
-- (his sprite, 08-10, appearing in animation 0, calling out in 3; his sounds, and none as a command is in) and its
-- differences:
--
-- - his commands give the crush the parameters 0x1207 (paralysis, its palette 7) and 0x2208 (confusion, its palette
--   8); without one it carries a move bug (code 0x0D) in a direction he draws as he appears (0x080BAE20: up, down,
--   left or right, 0x080BAE5C), which ../../rules/navicust/bugs takes;
-- - he goes 61 ticks after his crush, without the bow, in a puff (effect row 0x13);
-- - his crush stands 8 pixels left of its panel's center and 12 up (0x080D37A8), strikes the second time with 40
--   ticks left, and highlights nothing;
-- - his spawner (0x080BAF1A) keeps his controller's flag as his first related object.

local collision = require("../../rules/collision")
local navi_chips = require("../../lib/navi_chips")
local statuses = require("../../rules/status")

-- His crush's figure from its panel's center: its X's and Y's whole parts 8 and 12 less (two halfword stores).
local function place(me: Object)
    local p = me.pos
    local function less(v: number, n: number): number
        local whole = bit32.band(bit32.rshift(v, 16) - n, 0xFFFF)
        return int.i32(bit32.bor(bit32.lshift(whole, 16), bit32.band(v, 0xFFFF)))
    end
    me.pos = Vec3.new(less(p.x, 8), less(p.y, 12), p.z)
end

local CRUSH = require("@exelib/shademan/crush").make {
    id = "shademan/crush",
    sprite = asset.sprite("shademan"),
    sound = asset.sound("shade-crush"),
    attack = collision.piercing,
    hits = collision.hits_navis,
    place = place,
    second_at = 40,
    highlights = false,
}

-- The move bug's directions a draw picks (0x080BAE5C: up, down, left, right).
local DIRECTIONS: { number } = { 0x40, 0x80, 0x20, 0x10 }

local SHADEMAN = require("@exelib/shademan/navi").make {
    id = "shademan/navi",
    sprite = asset.sprite("shademan"),
    appear_anim = 0,
    call_anim = 3,
    appear = asset.sound("appear"),
    call = asset.sound("shade-call"),
    crush = CRUSH,
    commands = {
        {
            steps = { { "down" }, { "down", "right" }, { "right", "a" } },
            hit = { palette = 7, hit_mod = 0, status = statuses.paralyze_150 },
        },
        {
            steps = { { "down" }, { "down", "left" }, { "left", "a" } },
            hit = { palette = 8, hit_mod = 3, status = statuses.confuse_960 },
        },
    },
    uncommanded = function(): ShadeManHit
        local direction = DIRECTIONS[bit32.band(battle.rng_positive(), 3) + 1]
        return { palette = 0, hit_mod = 3, status = nil, bug = 0x0D, bug_arg = direction }
    end,
    bows = false,
    puff = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 },
}

-- 0x080BAF1A: ShadeMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(SHADEMAN.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    SHADEMAN.summoned(o, controller, palette)
    return o
end

-- What each ShadeMan chip states for its own controller: the controller's id (the original's effect object #0x15 for
-- all three: compat maps each to it) and his palette (0x080BADE0 by the chip's level: 0, the SP's 6, the DS's 14).
export type ChipSpec = { id: string, palette: number }

-- A ShadeMan chip's controller: the short course (the user warps out, 11 ticks, ShadeMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "ShadeMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
