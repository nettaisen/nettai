--!strict
-- SidBmbo's bamboo (attack object 0x44, 0x080D5C50: EXE4's own, its update EXE5's crack bomb's 0x080C920A but for its
-- constants), which the spawners' variant 5 puts out (0x080D5DDA): above the field (row 0) three columns ahead of the
-- user, 8 pixels lower, its shadow drawn with it, in its chip's palette (its parameter's twice), turned to its side.
-- On its first tick it swings (the sword's swing sound, 0x8E: its animation 2, 31 ticks), a hitbox on row 1 four ticks
-- in and on row 2 two ticks later (the wood spark, modifier 1, each lasting 20 ticks unless it hits); then its
-- animation 3 for 30 ticks and it goes in a puff (effect row 0x15). As the battle is over it goes in a puff where it
-- is. Its hitboxes are attacks hitting navis (0x080D5D84's 0x04050401).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local rock = require("../../lib/rock")
local sparks = require("../../lib/sparks")

local bamboo = {}

local SPRITE = asset.sprite("side-bamboo")
local SWING_SOUND = asset.sound("sword-swing")
-- Its swing: 31 ticks, the hitboxes at 27 and 25 left; then 30 ticks.
local SWING_TICKS, FIRST_HIT, SECOND_HIT, AFTER_TICKS = 0x1F, 0x1B, 0x19, 0x1E
local HITBOX_TICKS, HIT_MOD = 20, 1
local LOWER = 8

-- Its palette (twice its Param1, the chip's parameter).
type State = { palette: number }

local WAIT, SWING = 0x0, 0x4
local SWINGING, AFTER = 0x0, 0x4

-- 0x080D5D2C: the swing, its hitboxes on rows 1 and 2 of its column.
local function swinging(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SWING_SOUND)
        me.anim = 2
        me.timer = SWING_TICKS
        me.phase_init = 4
        return
    end
    local t = me.timer
    if t == FIRST_HIT or t == SECOND_HIT then
        local o = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y + (if t == FIRST_HIT then 1 else 2),
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.wood,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if o then
            o.timer = HITBOX_TICKS
        end
    end
    t -= 1
    me.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.phase, me.phase_init = AFTER, 0
    end
end

-- 0x080D5D88: 30 ticks more, then gone in a puff.
local function after(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t > 0 then
        return
    end
    battle.effect(me.pos, rock.effects.puff)
    me:set_lifecycle("destroy")
end

-- 0x080D5CC6.
local function tick(me: Object)
    if battle.over() then
        battle.effect(me.pos, rock.effects.puff)
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == WAIT then
        -- 0x080D5CFC: its count, then the swing.
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t < 0 then
            me.action = SWING
            me.phase, me.phase_init = 0, 0
        end
    elseif a == SWING then
        local p = me.phase
        if p == SWINGING then
            swinging(me)
        elseif p == AFTER then
            after(me)
        else
            error(string.format("SidBmbo's bamboo's phase %#x reads past its table", p))
        end
    else
        error(string.format("SidBmbo's bamboo's action %#x reads past its table", a))
    end
end

-- 0x080D5C74.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + LOWER * 0x10000, 0)
    sprite.palette = (me.state :: State).palette
    sprite:set_flip(me.alliance)
    me.timer = 0
    me:set_lifecycle("update")
    tick(me)
end

bamboo.kind = new.kind {
    id = "sidbmbo/bamboo",
    pool = "attack",
    state = { palette = "u8" },
    -- 0x080D5C50: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080D5DDA, the spawners' variant 5: the bamboo in `palette` (twice the chip's parameter) three columns ahead of
-- the user, on row 0, with the attack's element and damage word. (Its position is the registers until its init
-- places it.)
function bamboo.spawner(palette: number): (user: Object, element: number, damage: number) -> ()
    return function(user: Object, element: number, damage: number)
        local o = battle.spawn(bamboo.kind, Vec3.new(user.panel_y, element, 3 * user.facing))
        if o then
            o.panel_x, o.panel_y = user.panel_x + 3 * user.facing, 0
            o.element = element
            o.alliance = user.alliance
            o:set_damage_word(damage)
            local s = o.state :: State
            s.palette = palette
        end
    end
end

return bamboo
