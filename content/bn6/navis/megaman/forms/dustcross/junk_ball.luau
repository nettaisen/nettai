--!strict
-- DustCross's ball of junk (the original's attack object #0xB0,
-- `sub_80DB6A4`), its charged shot's (./charge) and DustMan's
-- (chips/dustman): it rolls forward from in front of the navi, 6 pixels a
-- tick, to the far column; where it meets the other side's body or object
-- it stops, and 5 ticks later hits its panel (cracking a solid one). The
-- charged shot's stands still while dimmed; DustMan's runs on.

local collision = require("../../../../rules/collision")
local regions = require("@common/regions")
local sparks = require("../../../../lib/sparks")

local junk_ball = {}

export type State = {
    -- It runs on while dimmed (the original's first parameter).
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("bn6:dustman")
local SOUND_LAND = asset.sound("bn6:land")

-- How far in front of its panel's center it starts, and how fast it rolls,
-- 16.16.
local AHEAD, SPEED = 0x12_0000, 0x6_0000

-- What stops it (`byte_80DB888`, by side): the other side's bodies and
-- objects, and neutral objects.
local STOPS = { [0] = 0x05800000, [1] = 0x0A800000 }

-- The phases, on its action.
local ROLL, BURST = 0, 4

-- Its hit: a thrown attack that breaks, at navis, with the breaking
-- spark.
local HIT_MOD = 3

-- `sub_80DB6D4`: in front of its panel, on the ground.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0x19
    me.anim_loaded = 0x19
    sprite:set_animation(0x19)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80DB864`: the other side's body or object on its panel.
local function blocked(me: Object): boolean
    return bit32.btest(field.flags(me.panel_x, me.panel_y), STOPS[me.alliance])
end

-- Stop rolling: on to the burst (only the phase byte is reset).
local function stop(me: Object)
    me.action = BURST
    me.phase = 0
end

-- `sub_80DB828`: its speed, and the ticks to the far column (none if it
-- is there already).
local function aim(me: Object)
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    local far = if me.alliance == 0 then 6 else 1
    if me.panel_x == far then
        me.vel = Vec3.new(0, me.vel.y, me.vel.z)
        me.timer = 0
        return
    end
    local x = field.center(far, 1)
    me.timer = int.tdiv(math.abs(x - me.pos.x), SPEED)
end

-- `sub_800E708`: `c` lies between `a` and `b` (either may equal it).
local function between(a: number, b: number, c: number): boolean
    if a == c or b == c then
        return true
    end
    if a <= b then
        return c > a and c <= b
    end
    return c >= b and c <= a
end

-- `sub_80DB75C`: roll to the far column, looking for a stop each time it
-- reaches or passes a panel's center.
local function roll(me: Object)
    if me.phase == 0 then
        me.phase = 4
        aim(me)
        if blocked(me) then
            return stop(me)
        end
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        return stop(me)
    end
    local center = field.center(me.panel_x, me.panel_y)
    local p = me.pos
    local old = p.x
    me.pos = Vec3.new(old + me.vel.x, p.y, p.z)
    if center ~= old and between(me.pos.x, old, center) and blocked(me) then
        return stop(me)
    end
    me:set_panel_from_coordinates()
end

-- `sub_80DB890`: the hit on its panel (a hitbox that runs while dimmed,
-- `sub_80C53A6`, with Param1 set).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.thrown_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if box and (me.state :: State).while_dimmed then
        box.run_while_dimmed = true
    end
end

-- `sub_80DB7B2`: 30 ticks of bursting; on the 5th the hit, and a solid
-- panel cracks.
local function burst(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 0x1A
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x19 then
        hit(me)
        if field.solid(me.panel_x, me.panel_y) then
            battle.shake_camera(2, 0x14)
            battle.play_sound(SOUND_LAND)
            field.crack(me.panel_x, me.panel_y)
        end
    elseif left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DB726`: gone once the battle is over; still while dimmed (unless
-- it runs on).
local function tick(me: Object)
    if battle.over() then
        me.lifecycle = "destroy"
        return
    end
    if not (me.state :: State).while_dimmed and battle.dimmed() then
        return
    end
    local a = me.action
    if a == ROLL then
        roll(me)
    elseif a == BURST then
        burst(me)
    else
        error(string.format("dust ball action %#x reads past its table", a))
    end
end

junk_ball.kind = define.kind {
    id = "bn6:dustcross/junk-ball",
    pool = "attack",
    state = { while_dimmed = "bool" },
    -- `sub_80DB6A4`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80DB800`: a ball from `owner` starting on (x, y), with the damage
-- word `damage`; DustMan's (`while_dimmed`) runs on while dimmed. Its
-- header runs while dimmed either way. (Its Z is the spawner's register
-- garbage, of which the init keeps the fraction.)
function junk_ball.spawn(owner: Object, x: number, y: number, element: number, damage: number, while_dimmed: boolean?): Object?
    local o = battle.spawn(junk_ball.kind, Vec3.zero)
    if o then
        (o.state :: State).while_dimmed = while_dimmed or false
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return junk_ball
