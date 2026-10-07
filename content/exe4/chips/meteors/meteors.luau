--!strict
-- EXE4's Meteors: the spawners' variant 0x11 (0x080EAE92: EXE5's 0x080E9DA2, the same code, which no EXE5 chip
-- uses) finds the target's panel and puts out the dropper there (effect object 0x81, 0x080EADF4: EXE5's 0x080E9D04,
-- the same code), which drops five meteors on it, one every 20 ticks.
--
-- The meteor is the shared falling meteor (attack object 0x56, 0x080D85AA: @exelib/meteors/falling_meteor, EXE5's
-- code) with EXE4's look: its sprite (0c-31), its sounds (0x146 falling, 0x70 landing), its burst (effect row 6), its
-- marker (left side 0: 0x080E5EEA sets no side), and its hit set on its panel where it lands whether it bursts there
-- or not (0x080D873A). Its row is the table 0x080D8580's row 6 (the dropper's, 0x080EAE56): hit modifier 0
-- (0x080D8642), the camera shaking. The rows' panel byte is 0 in all seven, so a meteor of EXE4's never cracks or
-- breaks its panel (0x080D871E).
--
-- The marker is the shared marker (effect object 0x35, 0x080E5E38: @exelib/meteors/marker): EXE4's lights only its own
-- panel (0x08009DA0), which is the shared one's single panel, its row 0 (0x080E5E28) the shared meteor's (45 ticks, lit
-- 5 of every 10).

local collision = require("../../rules/collision")
local falling_meteor = require("@exelib/meteors/falling_meteor")
local marker = require("@exelib/meteors/marker")
local panels = require("@exelib/panels")
local sparks = require("../../lib/sparks")

local meteors = {}

-- Row 6 of 0x080D8580: thrown, hitting navis, modifier 0, the camera shaking (its fifth byte set: 0x080D8716).
local ROW = falling_meteor.row { is = collision.thrown, hits = collision.hits_navis, hit_mod = 0, shake = 1, panel = "none" }

local METEOR = falling_meteor.make_kind("meteor", {
    sprite = asset.sprite("meteor"),
    sounds = { fall = asset.sound("justice-one"), land = asset.sound("hit-bomb-1") },
    spark = sparks.of_element,
    -- Effect row 6 (0x080E1698): 14-0a.
    burst = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 },
    marker = marker.make_kind("meteors/marker"),
    hit_on = "landing",
    marker_takes_side = false,
})

-- The dropper's meteors and the ticks between them (0x080EAE34, 0x080EAE4C).
local DROPS, EVERY = 5, 20

local SETUP, DROP = 0x0, 0x4

-- 0x080EAE1A: its first tick sets up the five drops; then a meteor at once and one every 20 ticks after, on its panel;
-- gone after the fifth.
local function tick(me: Object)
    if me.phase == SETUP then
        me.timer2 = DROPS
        me.timer = 0
        me.phase, me.phase_init = DROP, 0
        return
    end
    local t = me.timer - 1
    if t >= 0 then
        me.timer = t
        return
    end
    me.timer = EVERY
    -- 0x080D877C: the row its parameter word (r4), its position the registers until its init places it.
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    falling_meteor.spawn_kind(METEOR, ROW, me, me.panel_x, me.panel_y, me.element, damage, false,
        Vec3.new(me.panel_y, me.element, 0))
    local left = me.timer2 - 1
    me.timer2 = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

local DROPPER = new.kind {
    id = "meteors/dropper",
    pool = "effect",
    -- 0x080EADF4.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080EAE14
            me:set_lifecycle("update")
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- The panel filters of 0x0800FC48 by the user's side (0x0800FC84, 0x0800FC94): the target's side's panel holding the
-- target (its navi bit: 0x200000 for side 1, 0x400000 for side 0), else just the target's side's panel (the owner bit
-- 0x20 set for side 1, clear for side 0).
local HOLDING = { [0] = { require = 0x200020, forbid = 0 }, [1] = { require = 0x400000, forbid = 0x20 } }
local OWNED = { [0] = { require = 0x20, forbid = 0 }, [1] = { require = 0, forbid = 0x20 } }

-- 0x0800FC48: in the user's row, from the user's side's back edge on, the first of the other side's panels holding
-- its navi, else the first of the other side's panels; 0 for none.
local function target(user: Object): number
    local other, y = bit32.bxor(user.alliance, 1), user.panel_y
    local f = HOLDING[user.alliance]
    local x = panels.closest_in_row(other, y, f.require, f.forbid)
    if x == 0 then
        f = OWNED[user.alliance]
        x = panels.closest_in_row(other, y, f.require, f.forbid)
    end
    return x
end

-- 0x080EAE92: the spawner: the dropper on the target's panel in the user's row (none without one), with the attack's
-- element and damage word, of the user's side, not running while dimmed (0x080EAE6C: header flag 0x10 cleared). Its
-- position is the registers (the row, the element, the filter the search ended on), which nothing reads.
function meteors.spawn(user: Object, element: number, damage: number)
    local x = target(user)
    if x == 0 then
        return
    end
    local o = battle.spawn(DROPPER, Vec3.new(user.panel_y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, user.panel_y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = user.alliance
        o.run_while_dimmed = false
    end
end

return meteors
