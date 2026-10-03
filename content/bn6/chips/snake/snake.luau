--!strict
-- A snake (the original's attack object #0x73, `sub_80D2EBC`;
-- docs/engine/dimming-chip-effects.md §13): it rises out of its hole,
-- waits, then leaps at its target panel and strikes it, or flies on off
-- the field when it has none. Snake's nest (./nest) sends them out.

local regions = require("../../lib/regions")
local collision = require("../../rules/collision")
local trajectory = require("../../lib/trajectory")

local snake = {}

export type State = {
    -- The panel it leaps at (ExtraVars+0, +4; x 0xFF: none).
    target_x: number,
    target_y: number,
    -- The nest whose `snakes` count it is in (the original's pointer to
    -- the nest's counter).
    nest: Object?,
}

local SPRITE = asset.sprite("bn6:snake")
local SOUND_RISE = asset.sound("bn6:appear")
local SOUND_LEAP = asset.sound("bn6:aqua-needle-2")
-- Its strike (`sub_80C53A6`, 0x04050501): an attack hitting navis on the
-- target panel, 8 pixels up, with hit modifier 3, resolving while dimmed.
local STRIKE_SPARK = define.spark { sprite = asset.sprite("bn6:pink-flash"), anim = 0, palette = 0 }
local STRIKE_Z, STRIKE_HIT_MOD = 0x8_0000, 3
-- Rising two pixels a tick up to 10; then 20 ticks; then the leap, 16
-- pixels a tick.
local RISE, RISE_TO, WAIT, SPEED = 0x2_0000, 10, 0x14, 0x10_0000
local ANIM_LEAP = 1

local function counter_add(me: Object, n: number)
    local nest = (me.state :: State).nest
    if nest and nest.active then
        local s = nest.state :: { snakes: number }
        s.snakes += n
    end
end

-- `sub_80D3058`: 16 pixels a tick toward the target panel's center, or
-- straight ahead without one.
local function aim(me: Object)
    local s = me.state :: State
    local x, y = field.center(me.panel_x, me.panel_y)
    if s.target_x == 0xFF then
        me.vel = Vec3.new(me.facing * SPEED, 0, me.vel.z)
        return
    end
    local tx, ty = field.center(s.target_x, s.target_y)
    local angle = trajectory.angle(int.i32(tx - x), int.i32(ty - y))
    local vx, vy = trajectory.toward(angle, SPEED)
    me.vel = Vec3.new(vx, vy, me.vel.z)
end

-- `sub_80D2F70`: rise out of the hole (its phase is the init flag).
local function rise(me: Object)
    if me.phase == 0 then
        me.vel = Vec3.new(me.vel.x, me.vel.y, RISE)
        battle.play_sound(SOUND_RISE)
        me.phase = 4
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
    if bit32.band(bit32.rshift(me.pos.z, 16), 0xFFFF) >= RISE_TO then
        me.action, me.phase, me.phase_init = 4, 0, 0
    end
end

-- `sub_80D2FA0`: 20 ticks.
local function wait(me: Object)
    if me.phase == 0 then
        me.timer = WAIT
        me.phase = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me.action, me.phase, me.phase_init = 8, 0, 0
    end
end

-- `sub_80D3004`: fly; on the target panel, strike it and go.
local function fly(me: Object)
    local s = me.state :: State
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y + me.vel.y, p.z)
    me:set_panel_from_coordinates()
    if me.panel_x ~= s.target_x or me.panel_y ~= s.target_y then
        return
    end
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = STRIKE_Z,
        region = regions.single,
        hit_effect = STRIKE_SPARK,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = STRIKE_HIT_MOD,
    })
    if h then
        h.run_while_dimmed = true
    end
    me:set_lifecycle("destroy")
end

-- `sub_80D2FC2`: the leap, by phase: its animation; the sound at the
-- animation's end; the flight.
local function leap(me: Object)
    local phase = me.phase
    if phase == 0 then
        me.anim = ANIM_LEAP
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            battle.play_sound(SOUND_LEAP)
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        fly(me)
    else
        error(string.format("Snake's snake: leap phase %#x reads past off_80D2FD4", phase))
    end
end

-- `sub_80D2F3C`: gone once off the field; else the action and the
-- sprite.
local function tick(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == 0 then
        rise(me)
    elseif a == 4 then
        wait(me)
    elseif a == 8 then
        leap(me)
    else
        -- off_80D2F60's fourth entry is a data word, not a routine.
        error(string.format("Snake's snake: action %#x reads past off_80D2F60", a))
    end
    me:update_sprite()
end

-- `sub_80D2EDC`: on a panel of the field, it joins the nest's count and
-- shows on its hole, aimed; off it, it is freed at once.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    counter_add(me, 1)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    aim(me)
    me:set_lifecycle("update")
    tick(me)
end

snake.kind = define.kind {
    id = "bn6:snake/snake",
    pool = "attack",
    state = { target_x = "u32", target_y = "u32", nest = "object" },
    -- `sub_80D2EBC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- sub_80D3048: out of the nest's count; freed.
            counter_add(me, -1)
            me:free()
        end
    end,
}

-- `sub_80D30A4`: a snake of `nest`'s side and facing in the hole (`x`,
-- `y`), leaping at (`tx`, `ty`) (`tx` 0xFF: none), with `nest`'s damage
-- word. (Spawned at the spawner's registers; its init places it.)
function snake.spawn(nest: Object, x: number, y: number, tx: number, ty: number): Object?
    local o = battle.spawn(snake.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        local s = o.state :: State
        s.target_x, s.target_y, s.nest = tx, ty, nest
        o:set_damage_word(bit32.bor(nest.damage, bit32.lshift(nest.stamina, 16)))
        o.alliance, o.flip = nest.alliance, nest.flip
    end
    return o
end

return snake
