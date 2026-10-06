--!strict
-- A die (EXE5's attack object #5, 0x080C4F2C; EXE5's own: EXE6's `sub_80C55B0` family is another object), thrown at a
-- panel: it arcs there (rising 4 pixels a tick, falling 0x3000 a tick, as long as that takes), hitting what it
-- meets while low; landed on a panel that isn't solid it is gone in a puff, else it bounces, shows its face (1 to
-- 6, or 9's own) for 60 ticks, flashing white in the last 50, and bursts: a hit over the 3x3 around it worth its
-- face times its damage. Its HP and the enemy's bodies and objects it touches blow it up first, as does the battle's
-- end. NumberMan's (chips/numbrman/navi) runs while dimmed, an attack at navis; the others' (the 0x080EE804,
-- 0x080F1652 and 0x080FA66A throws) a body the other side can hit, standing still while dimmed.
--
-- Its markers over its target (./marker) show in the operation battle only.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local trajectory = require("../../lib/trajectory")
local marker = require("../../lib/navi_chips/marker")

local dice = {}

export type State = {
    -- Its face (Param0: 1 to 6, or 9), whether it runs while dimmed (Param1), its target panel (Param2, Param3).
    face: number,
    dimmed: boolean,
    target_x: number,
    target_y: number,
    -- Its hits' status and hit modifier (+0x64, +0x68).
    status: Status?,
    hit_mod: number,
    -- The object waiting on it (+0x60, the flag its spawner gave it): its `busy`, cleared when it goes.
    holder: Object?,
}

export type Throw = {
    -- Its face (1 to 6, or 9), its target panel, and whether it is NumberMan's (an attack at navis running while
    -- dimmed: its Param1).
    face: number,
    target_x: number,
    target_y: number,
    dimmed: boolean,
    -- Its HP (its packed word's low byte), its hits' status (the next) and hit modifier (the third).
    hp: number,
    status: Status?,
    hit_mod: number,
    damage: number,
    holder: Object?,
}

local SPRITE = asset.sprite("dice")
local SOUND_BURST = asset.sound("hit-bomb-1")
local SOUND_BOUNCE = asset.sound("bug-bomb-land")
local SOUND_ROLL = asset.sound("target-move")
-- The effect table's rows: 0 (the explosion), 0x24 (9's: its palette 1), 2 (the puff off the field), 0x14 (the
-- battle's end).
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local EXPLOSION_9 = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 }
local PUFF = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 }
local GONE = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
-- What blows it up (0x080C507C): the other side's bodies and objects, by its side.
local ENEMY = { [0] = 0x0580_0000, 0x0A80_0000 }
local RISE, GRAVITY = 0x4_0000, -0x3000
local LOW = 0x19
local SHOW = 0x3C

local function done(me: Object)
    local h = (me.state :: State).holder
    if h then
        (h.state :: any).busy = false
    end
end

-- Gone: its hit off, its holder freed.
local function go(me: Object)
    me.collision:set_region(nil)
    done(me)
    me:set_lifecycle("destroy")
end

-- `sub_8001288`: the ticks a rise of `vz` takes to come back down at `gravity` (one more when the division's
-- remainder is at least half the quotient: the BIOS leaves the quotient's magnitude where the game meant the rise),
-- and the velocity that crosses (dx, dy) in them.
local function arc(me: Object, x: number, y: number, vz: number, gravity: number): (number, number, number)
    local dx, dy = int.i32(x - me.pos.x), int.i32(y - me.pos.y)
    local ticks, rem, magnitude = trajectory.div(vz * 2, -gravity)
    if int.i32(rem - bit32.rshift(magnitude, 1)) >= 0 then
        ticks += 1
    end
    local vy = trajectory.div(dy, ticks)
    local vx = trajectory.div(dx, ticks)
    return ticks, vx, vy
end

-- Action 0 (0x080C5084): the flight: its hit on while it is under 25 pixels up; landed (its ticks run out), on its
-- target, gone off a panel that isn't solid; gone off the field on the way.
local function fly(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local cx, cy = field.center(s.target_x, s.target_y)
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, RISE)
        local ticks, vx, vy = arc(me, cx, cy, RISE, GRAVITY)
        me.timer2 = int.u16(ticks)
        me.vel = Vec3.new(vx, vy, RISE)
        marker.spawn(me, s.target_x, s.target_y, ticks, 1, false)
        me.phase_init = 4
    end
    local v = me.vel
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + vz))
    if bit32.band(bit32.rshift(me.pos.z, 16), 0xFFFF) < LOW then
        me.collision:set_region(regions.single)
    else
        me.collision:set_region(nil)
    end
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left < 0 then
        me.panel_x, me.panel_y = s.target_x, s.target_y
        local q = me.pos
        me.pos = Vec3.new(q.x, q.y, 0)
        me:set_coordinates_from_panel()
        if field.solid(me.panel_x, me.panel_y) then
            me:set_action(4)
            return
        end
        me.collision:set_region(nil)
        battle.effect(me.pos, PUFF)
        done(me)
        me:set_lifecycle("destroy")
        return
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        go(me)
    end
end

-- Action 4 (0x080C5154): a bounce of 3 pixels a tick (its sound), its marker each tick; on to the face once it
-- comes down (its height left as it is).
local function bounce(me: Object)
    marker.spawn(me, me.panel_x, me.panel_y, 1, 1, false)
    if me.phase_init == 0 then
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, 0x3_0000)
        battle.play_sound(SOUND_BOUNCE)
        me.phase_init = 4
    end
    local v = me.vel
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(p.x, p.y, z)
    if z <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080C5190): its face (9's own animation 7, else the face's less one) for 60 ticks, its marker counting
-- them; white every other two ticks in the last 50.
local function show(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = if s.face == 9 then 7 else s.face - 1
        me.timer = SHOW
        marker.spawn(me, me.panel_x, me.panel_y, SHOW, 0xF, true)
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    me.sprite.white = false
    if left < 0 then
        me:set_action(0xC)
        return
    end
    if left < 50 and bit32.band(left, 2) ~= 0 then
        me.sprite.white = true
    end
end

-- Action 0xC (0x080C51E4): the burst: a hit over the 3x3 around it (region 0x0F, no spark, target 5, self 4), worth
-- its face times its damage (the damage's top bit kept), with its status and hit modifier (resolving while dimmed
-- when it runs then); the explosions over the 3x3; its sound; gone.
local function burst(me: Object)
    local s = me.state :: State
    me.collision:set_region(nil)
    local damage = bit32.band(s.face * me.damage, 0x7FFF) + bit32.band(me.damage, 0x8000)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.square,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = damage,
        stamina = me.stamina,
        hit_mod = s.hit_mod,
        status = s.status,
    })
    if box and s.dimmed then
        box.run_while_dimmed = true
    end
    battle.region_effects(me.panel_x, me.panel_y, regions.square, me.alliance, if s.face == 9 then EXPLOSION_9 else EXPLOSION)
    battle.play_sound(SOUND_ROLL)
    done(me)
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = fly, [4] = bounce, [8] = show, [0xC] = burst }

-- 0x080C4FDE: its hits resolved; gone at the battle's end (a puff), blown up by its HP running out or an enemy's
-- body or object (the explosion, its sound); else its action (unless dimmed, for one that stands still then).
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, GONE)
        go(me)
        return
    end
    if me:take_damage(0) < 0 or bit32.btest(c.hit_flags, ENEMY[me.alliance]) then
        battle.effect(me.pos, if s.face == 9 then EXPLOSION_9 else EXPLOSION)
        battle.play_sound(SOUND_BURST)
        go(me)
        return
    end
    if s.dimmed or not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("a die: action %#x reads past its table (0x080C506C)", me.action))
        end
        action(me)
    end
    c:present()
end

-- 0x080C4F60: shown, its face's spin (animation 6), 9's palette 1; NumberMan's an attack at navis (rows 4 and 5),
-- the others' a body the other side hits (rows 0x13 and 0x14); no spark; then its first update.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 6, 0xFF
    me.element = 0
    if s.face == 9 then
        sprite.palette = 1
    end
    if not me:create_collision() then
        done(me)
        me:free()
        return
    end
    if s.dimmed then
        me:setup_collision(collision.attack, collision.hits_navis, s.hit_mod)
    else
        me:setup_collision(collision.own_body, collision.hit_by_other_side, s.hit_mod)
    end
    local c = me.collision
    if s.status then
        c.status_base = s.status
    end
    c:set_hit_effect(nil)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

dice.kind = new.kind {
    id = "numbrman/dice",
    pool = "attack",
    state = {
        face = "u8",
        dimmed = "bool",
        target_x = "u8",
        target_y = "u8",
        status = "status",
        hit_mod = "u8",
        holder = "object",
    },
    -- 0x080C4F2C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).dimmed then
            me:update_sprite_while_dimmed()
        elseif not battle.dimmed() then
            me:update_sprite()
        end
    end,
}

-- 0x080C525E: a die of `owner`'s at `pos`, as `throw` says, of its side and facing, running while dimmed (its
-- update decides what it does then); its holder (if any) busy until it goes.
function dice.spawn(owner: Object, pos: Vec3, throw: Throw): Object?
    local o = battle.spawn(dice.kind, pos)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.face, s.dimmed = throw.face, throw.dimmed
    s.target_x, s.target_y = throw.target_x, throw.target_y
    s.status, s.hit_mod = throw.status, throw.hit_mod
    o.hp, o.max_hp = throw.hp, throw.hp
    o.related1 = owner
    o.alliance, o.flip = owner.alliance, owner.flip
    o:set_damage_word(throw.damage)
    o.run_while_dimmed = true
    s.holder = throw.holder
    if throw.holder then
        ((throw.holder :: Object).state :: any).busy = true
    end
    return o
end

return dice
