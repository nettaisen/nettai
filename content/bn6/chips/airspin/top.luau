--!strict
-- AirSpin's whirlwind top (attack object #0x9B, `sub_80D88E0`): a field
-- object (its side's class-1 one) with 400 HP. Set down on a solid panel
-- with no body on it, it rolls forward at 4 pixels a tick toward the next
-- panel while that one is open (solid, nothing standing on it), at most 4
-- panels, then stands 14 ticks and spins: it hits the ring around its
-- panel (a one-tick hit with the wind's gust) every 10 ticks, its spins'
-- times, one more for each wind hit it takes, and blows away. It breaks
-- when its HP runs out or a body, a neutral object or a type-2 attack
-- touches it; a chip can remove it (absorbed, or blinking out). It keeps
-- running while dimmed, but only acts outside dimming.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")

local top = {}

export type State = {
    -- The X center of the panel it rolls toward (ExtraVars+0).
    target_x: number,
    -- Ticks between its hits (ExtraVars+4).
    interval: number,
    -- Spins left (the original's second parameter), and its palette
    -- (before the times 4: its first).
    spins: number,
    palette: number,
}

-- Its sprite: animation 0 standing, 1 spinning.
local SPRITE = asset.sprite("air-spin")
local HIT_MOD = 0x03
-- Rolling speed (16.16), panels it rolls at most, HP.
local SPEED, MAX_PANELS, HP = 0x4_0000, 4, 0x190
-- Its identity: a field object, with the look something that takes its
-- place shows (DustMan's junk).
local IDENTITY = define.identity {
    id = "airspin-top",
    class = "field_object",
    version = 0,
    ai_index = 1,
    look = { sprite = asset.sprite("air-spin"), anim = 1, palette = 0, shadow = false },
}
-- Its field-object class, and how it looks absorbed (the original's
-- obstacle kind 0x0C: its own sprite).
local CLASS = 1
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
-- The spin's gust (effect 0x43), and its palette offset.
local GUST = define.effect { sprite = asset.sprite("moon-blade"), anim = 0, palette = 0 }
local GUST_PALETTE = 2
local SOUND_SET_DOWN, SOUND_SPIN = asset.sound("set-down"), asset.sound("spin")
local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")
-- Panel flags: solid (0x10); where it can't be set down (bodies:
-- 0x3800000); what stops its rolling (bodies and neutral objects:
-- 0xF800000).
local SOLID, BODIES, BLOCKERS = 0x10, 0x03800000, 0x0F800000
-- Hits that break it at once: bodies, neutral objects, type 2.
local CRUSHING = 0x0F800002
-- Hits that make it spin once more: hit flag 0x800, or a wind hit
-- (secondary element 0x20).
local EXTRA_SPIN_HIT, WIND = 0x800, 0x20
-- Its hit: the ring around its panel, no spark, a push.
local HIT_MOD_SPIN = 1

-- The actions.
local ROLL, STAND, SPIN, BLOW_AWAY, REMOVED = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80D8C10`: it can roll onto (x, y): on the field, solid, nothing
-- standing there.
local function open(x: number, y: number): boolean
    local f = field.flags(x, y)
    return f ~= 0 and not bit32.btest(f, BLOCKERS) and bit32.btest(f, SOLID)
end

-- `sub_80D8BBC`: the next panel forward is where it rolls, and its center
-- the X it rolls to.
local function aim(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    me.future_panel_x, me.future_panel_y = x, y
    local cx = field.center(x, y)
    local s = me.state :: State
    s.target_x = cx
end

-- `sub_800E6E8`: `center` lies past `from`, up to and including `to`.
local function crossed(to: number, from: number, center: number): boolean
    if to > from then
        return center > from and center <= to
    end
    return center > to and center <= from
end

-- `sub_800E24C`: the panel under the object's coordinates.
local function panel_under(me: Object): (number, number)
    local p = me.pos
    return int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28), int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
end

-- A puff of wind at its position, raised by `lift` pixels.
local function puff(me: Object, lift: number)
    battle.effect(me.pos + Vec3.px(0, 0, lift), effects.puff)
end

-- Gone: out of the target tracking and the field-object registry, its
-- region cleared, destroyed at the next update.
local function leave(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- ---- The actions ---------------------------------------------------------------

-- Stop rolling: onto its panel's center.
local function stop(me: Object)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me:set_action(STAND)
end

-- `sub_80D8A3C`: roll toward the next panel while it is open; past its
-- center, aim at the one after, for at most 4 panels.
local function roll(me: Object)
    if me.phase_init == 0 then
        if not open(me.future_panel_x, me.future_panel_y) then
            return stop(me)
        end
        me.phase_init = 4
    end
    local s = me.state :: State
    local x = int.i32(me.pos.x + me.vel.x)
    if crossed(x, me.pos.x, s.target_x) then
        aim(me)
        if not open(me.future_panel_x, me.future_panel_y) then
            return stop(me)
        end
        local left = me.timer2 - 1
        me.timer2 = left
        if left <= 0 then
            return stop(me)
        end
    end
    -- (The panel under its coordinates stops it if it is open and not the
    -- one it is on, which rolling never makes.)
    local px, py = panel_under(me)
    if open(px, py) and me.panel_x ~= px then
        return stop(me)
    end
    local p = me.pos
    me.pos = Vec3.new(x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80D8AB6`: stand 14 ticks, then spin every 10.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x0F
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        -- sub_80D8C06
        (me.state :: State).interval = 0x0A
        me:set_action(SPIN)
    end
end

-- `sub_80D8BD4`: a hit around its panel, and the gust.
local function gust(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        region = regions.ring,
        -- None.
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.pushing,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD_SPIN,
    })
    battle.effect(me.pos, GUST, me.alliance, GUST_PALETTE)
end

-- `sub_80D8ADC`: spin, hitting at once and then every `interval` ticks,
-- its spins' times.
local function spin(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local s = me.state :: State
    me.timer = s.interval
    gust(me)
    battle.play_sound(SOUND_SPIN)
    local left = s.spins - 1
    s.spins = left
    if left <= 0 then
        me:set_action(BLOW_AWAY)
    end
end

-- `sub_80D8B18`: a puff, and gone 3 ticks later.
local function blow_away(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
        puff(me, 0)
        me.timer = 4
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        leave(me)
    end
end

-- `sub_80D8B52`: removed by a chip: absorbed, it flies to the absorbing
-- navi; blinking out, it blinks first; otherwise it goes in a puff.
local function removed(me: Object)
    if me:obstacle_flag("absorbed") then
        obstacle.fly_to_absorber(me, ABSORBED)
    else
        local b = obstacle.blink_out(me)
        if b == "blinking" then
            return
        elseif b == "no" then
            puff(me, 0x0C)
        end
    end
    leave(me)
end

local ACTIONS: { [number]: (Object) -> () } = {
    [ROLL] = roll,
    [STAND] = stand,
    [SPIN] = spin,
    [BLOW_AWAY] = blow_away,
    [REMOVED] = removed,
}

-- ---- The lifecycle -------------------------------------------------------------

-- `sub_80D8988`: take its hits; break, blow away off solid ground, or act
-- (outside dimming, unless a chip just removed it).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    local broken = me:take_damage(0) < 0 or bit32.btest(c.hit_flags, CRUSHING)
    if broken then
        battle.effect(me.pos, effects.explosion)
        battle.play_sound(SOUND_HIT_BOMB)
        leave(me)
    else
        if bit32.btest(c.hit_flags, EXTRA_SPIN_HIT) or bit32.btest(c.damage_elements, WIND) then
            local s = me.state :: State
            s.spins += 1
        end
        if not field.solid(me.panel_x, me.panel_y) then
            puff(me, 0)
            leave(me)
        else
            local run = not battle.dimmed()
            if me:obstacle_flag("removed") and me.action ~= REMOVED then
                me.action = REMOVED
                me.phase = 0
                me.phase_init = 0
                run = true
            end
            if run then
                local act = ACTIONS[me.action]
                if act == nil then
                    error(string.format("air spin action %#x reads past its table", me.action))
                end
                act(me)
            end
        end
    end
    c:present()
end

-- `sub_80D8908`: on a solid panel with no body, set down and roll;
-- otherwise a puff, and gone.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local f = field.flags(me.panel_x, me.panel_y)
    if not bit32.btest(f, SOLID) or bit32.btest(f, BODIES) then
        puff(me, 0x10)
        obstacle.unregister(me)
        me:free()
        return
    end
    -- sub_8011504
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        -- (The game goes on without collision data and reads through a
        -- null pointer at the first update.)
        error("air spin: no collision slot for the whirlwind (sub_8011504)")
    end
    me:setup_collision(collision.neutral_body, collision.everything, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    me.timer2 = MAX_PANELS
    me.identity = IDENTITY
    me.hp = HP
    me.max_hp = HP
    aim(me)
    sprite.palette = (me.state :: State).palette * 4
    battle.play_sound(SOUND_SET_DOWN)
    me:set_lifecycle("update")
    tick(me)
end

top.kind = define.kind {
    id = "airspin/top",
    pool = "attack",
    state = { target_x = "i32", interval = "i32", spins = "u8", palette = "u8" },
    -- `sub_80D88E0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        if me.active and not battle.dimmed() then
            me:update_sprite()
        end
    end,
}

-- `sub_80D8B96`: a whirlwind top for `owner` on (x, y) in `palette`,
-- spinning `spins` times, with the damage word `damage`. It runs while
-- dimmed and is its side's class-1 field object at once. Its position is
-- the spawner's registers (the panel Y, the element and 0) until its init
-- places it.
function top.spawn(owner: Object, x: number, y: number, element: number, palette: number, spins: number, damage: number): Object?
    local o = battle.spawn(top.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
        local s = o.state :: State
        s.palette = palette
        s.spins = spins
        obstacle.register(o, o.alliance, CLASS)
    end
    return o
end

-- What AirSpin sends: a top in `palette` that spins `spins` times (the
-- original's variant 0, with its first two parameters).
function top.sender(palette: number, spins: number): AirSpinSender
    return {
        palette = palette,
        spins = spins,
        send = function(user: Object, x: number, y: number, element: number, damage: number): Object?
            return top.spawn(user, x, y, element, palette, spins, damage)
        end,
    }
end

return top
