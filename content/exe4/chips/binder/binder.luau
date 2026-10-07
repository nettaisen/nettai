--!strict
-- EXE4's binder (attack object 0x2D, 0x080D2568: EXE4's own), which the Binders throw (the throw's variant 3, its
-- thrower 0x080D2748). From the throw's release point it hops along its row, each hop 26 ticks in an arc that falls
-- 0xA000 a tick (aimed by `sub_8001330`, @exelib/trajectory): two panels toward the other side, or, after its first
-- landing, in place while the other side's navi stands on its panel. Each landing hits its panel (16 pixels up: an
-- attack hitting navis, modifier 1, the heavy spark); on a solid panel it shows its landing animation 4 ticks and hops
-- on with its sound (0xC3), on a hole it goes in a puff (effect row 0x15, 16 pixels up). Halfway through its fourth
-- hop, or as the battle is over, it goes in a puff, and so it does past the field's sides. Its sprite (04-0A) is in
-- its chip's palette, turned to its side, its shadow on the ground.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local binder = {}

local SPRITE = asset.sprite("binder")
local SOUND = asset.sound("binder-hop")
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }

-- A hop: 26 ticks, falling 0xA000 a tick (0x080D2764), two panels ahead.
local HOP_TICKS, GRAVITY, HOP = 26, -0xA000, 2
-- Halfway through a hop (its count at 13) it looks at the battle and its landings: the fourth hop goes there.
local HALFWAY, LAST_HOP = 13, 3
-- The field's sides: 140 pixels either side of its middle.
local REACH = 140
-- Its landing: its hit 16 pixels up, modifier 1; its landing animation for 4 ticks.
local HIT_Z, HIT_MOD, LAND_TICKS = 0x10_0000, 1, 4
-- Its animations: hopping, its first hop's and landed.
local ANIM_HOP, ANIM_FIRST, ANIM_LANDED = 1, 2, 3
-- The other side's navi on a panel, by its side (0x080D2740).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
-- A puff's height over it.
local PUFF_Z = 0x10_0000

local HOPPING, LANDED = 0x0, 0x4

-- Its palette (its chip's) and its landings (+0x07).
type State = { palette: number, landings: number }

-- Gone in a puff (0x080D269E, 0x080D2700).
local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + PUFF_Z)), PUFF)
    me:set_lifecycle("destroy")
end

-- 0x080D271C: whether the other side's navi stands on its panel.
local function enemy_here(me: Object): boolean
    return bit32.btest(field.flags(me.panel_x, me.panel_y), ENEMY_BODY[me.alliance])
end

-- Action 0 (0x080D25EC): aimed on its first tick (in place, or two panels on); halfway, the battle and its landings
-- looked at; each tick on along its arc (its row: the aim's Y velocity is never read) until it lands on its panel's
-- center; off the field's sides, gone.
local function hop(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local x, y = me.panel_x + HOP * me.facing, me.panel_y
        if s.landings ~= 0 and enemy_here(me) then
            x = me.panel_x
        end
        local px, py = field.center(x, y)
        me.vel = trajectory.in_ticks(me.pos, px, py, 0, HOP_TICKS, GRAVITY)
        me.timer = HOP_TICKS
        me.phase_init = 4
    end
    if me.timer == HALFWAY and (battle.over() or s.landings >= LAST_HOP) then
        puff(me)
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me:update_collision_panels()
        s.landings += 1
        me.action = LANDED
        me.phase, me.phase_init = 0, 0
    else
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
        me:set_panel_from_coordinates()
        me:update_collision_panels()
    end
    local x = int.i16(bit32.rshift(me.pos.x, 16))
    if x + REACH < 0 or x + REACH >= 2 * REACH then
        puff(me)
    end
end

-- Action 4 (0x080D26B6): on its first tick its hit; on a solid panel its landing animation for 4 ticks, then its sound
-- and the next hop; on a hole, gone.
local function landed(me: Object)
    if me.phase_init == 0 then
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = HIT_Z,
            region = regions.single,
            hit_effect = sparks.heavy,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if not field.solid(me.panel_x, me.panel_y) then
            puff(me)
            return
        end
        me.anim = ANIM_LANDED
        me.timer = LAND_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    battle.play_sound(SOUND)
    me.anim = ANIM_HOP
    me.action = HOPPING
    me.phase, me.phase_init = 0, 0
end

-- 0x080D25D2.
local function act(me: Object)
    local a = me.action
    if a == HOPPING then
        hop(me)
    elseif a == LANDED then
        landed(me)
    else
        error(string.format("a binder's action %#x reads past 0x080D25E4", a))
    end
end

-- 0x080D258C: its sprite (its shadow on the ground), shown in its first hop's animation in its palette, turned to its
-- side, on the panel under it; no landings yet; then its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = ANIM_FIRST, 0xFF
    sprite.palette = (me.state :: State).palette
    sprite:set_flip(me.alliance)
    me:set_panel_from_coordinates()
    local s = me.state :: State
    s.landings = 0
    me:set_lifecycle("update")
    act(me)
end

binder.kind = new.kind {
    id = "binder/binder",
    pool = "attack",
    state = { palette = "u8", landings = "u8" },
    -- 0x080D2568: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080D2748, the throw's variant 3: a binder in `palette` (twice the chip's parameter) at the release point, of the
-- user's side, with the damage word; its element the byte the thrower stores from the register holding the point's Y
-- (its low byte: none for a navi at rest).
function binder.thrower(palette: number): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(binder.kind, pos)
            if o then
                o.element = bit32.band(pos.y, 0xFF)
                o.alliance = user.alliance
                o:set_damage_word(damage)
                local s = o.state :: State
                s.palette = palette
            end
            return o
        end,
    }
end

return binder
