--!strict
-- DeathPhoenix's fire on a panel (EXE5's effect object #0x89, 0x080EA5E4; EXE5's own): it rises there (its sprite's
-- animation 0, then 1 for 10 ticks), then burns for 40 ticks: a hit on its panel lasting 25 ticks at once, one on
-- the panel beside it lasting 31 ticks 16 ticks later (ahead of its side, or back toward the phoenix for a far
-- fire), and a flame (./flame) leaping off it every 4 ticks. Then it goes.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local regions = require("@exelib/regions")
local flame = require("./flame")

local fire = {}

export type State = {
    -- Far from the phoenix (its Param0): the second hit and the flames go back toward it.
    far: boolean,
}

local SPRITE = asset.sprite("phoenix-flame")
local SOUND_RISE = asset.sound("col-army")
local SOUND_BURN = asset.sound("phoenix-burn")
local RISE = 0xA
-- The flames: one every 4 ticks, ten of them.
local FLAME_EVERY, FLAMES = 4, 10
-- The hits: at the burn's start on its panel for 25 ticks, 16 ticks in beside it for 31.
local HIT_Z = 0x10_0000

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- `sub_80C53A6` (EXE5's 0x080C4DC6): a hit on (x, y) lasting `ticks`, running while dimmed (region 1, hit effect 0,
-- target 5, self 0xA, hit modifier 3, 16 pixels up).
local function hit(me: Object, x: number, y: number, ticks: number)
    local damage = damage_word(me)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = HIT_Z,
        region = regions.single,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
        box.timer = ticks
    end
end

-- Action 0 (0x080EA654): 10 ticks once it has risen.
local function start(me: Object)
    me.timer = RISE
    me:set_action(4)
end

-- Action 4 (0x080EA662): animation 0 to its end, then 1; 10 ticks of it, and the burn.
local function rise(me: Object)
    if me.anim == 0 then
        if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
            return
        end
        me.anim = 1
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.timer2 = 0
    me:set_action(8)
    battle.play_sound(SOUND_BURN)
end

-- Action 8 (0x080EA696): the hits at its 1st and 17th ticks (the second beside it: `object_getFlipDirection` of
-- its side and its Param0); a flame every 4 ticks; gone after the tenth.
local function burn(me: Object)
    local s = me.state :: State
    local t = me.timer2
    me.timer2 = int.u16(t + 1)
    if t == 0 then
        hit(me, me.panel_x, me.panel_y, 0x19)
    elseif t == 0x10 then
        local dir = if bit32.bxor(me.alliance, if s.far then 1 else 0) == 0 then 1 else -1
        hit(me, me.panel_x + dir, me.panel_y, 0x1F)
    end
    if me.phase == 0 then
        me.phase = 1
        flame.spawn(me, s.far)
        me.timer = FLAME_EVERY
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.phase = 0
    me.phase_init += 1
    if me.phase_init >= FLAMES then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = start, [4] = rise, [8] = burn }

local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("DethPhnx's fire: action %#x reads past its table (0x080EA648)", me.action))
    end
    action(me)
end

-- 0x080EA608: on its panel, on the ground, animation 0 with its shadow, facing its object's way (0x0801892E's end),
-- all of it under the objects; its sound; then its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.under_objects = true
    me.visible = true
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.play_sound(SOUND_RISE)
    me:set_lifecycle("update")
    tick(me)
end

fire.kind = new.kind {
    id = "dethphnx/fire",
    pool = "effect",
    state = { far = "bool" },
    -- 0x080EA5E4.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080EA71C: a fire of `phoenix`'s on (x, y), `far` from it, with its damage word, of its side and facing. (Its
-- position is the spawner's registers, which its init overwrites.)
function fire.spawn(phoenix: Object, x: number, y: number, far: boolean, damage: number): Object?
    local o = battle.spawn(fire.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(damage)
        o.alliance, o.flip = phoenix.alliance, phoenix.flip
        o.related1 = phoenix;
        (o.state :: State).far = far
    end
    return o
end

return fire
