--!strict
-- The Anubis statue (the original's attack object #0x55, `sub_80CF0D0`;
-- docs/engine/dimming-chip-effects.md §7): Anubis's and PoisPhar's. It
-- drops onto the panel in front of the user; on anything but free solid
-- ground it breaks at once. Standing, it drains every enemy panel's navi
-- 1 HP each period (Anubis's 6 ticks, PoisPhar's every tick), with bubbles
-- rising over the enemy's area every 16 ticks, for its lifetime (a field
-- obstacle's, counting only ticks it runs) or until time is up.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

local statue = {}

export type State = {
    variant: AnubisVariant,
    -- Ticks to the next bubble (ExtraVars+4).
    bubble_in: number,
}

local SPRITE = asset.sprite("anubis")
-- It drops from 160 pixels, 8 a tick.
local HEIGHT, FALL_SPEED = 0xA0_0000, 0x8_0000
local SOUND_LAND = asset.sound("land")
local SOUND_BREAK = asset.sound("hit-bomb-1")
-- Where it can stand: a solid panel with nothing on it.
local SOLID, OCCUPIED = 0x10, 0x0F88_0080
-- What it is and reaches: a field obstacle of its side, hit modifier 3.
local IS, HITS, HIT_MOD = collision.field_object, collision.hit_by_other_side, 3
-- Its kind among absorbed obstacles.
local ABSORBED_KIND = 3
-- A bubble every 16 ticks, on one of the other side's panels but its own
-- (`byte_80CF364`, by side), in palette 1.
local BUBBLE_EVERY = 0x10
local OTHER_SIDE: { [number]: { number } } = { [0] = { 0x20, 0 }, [1] = { 0, 0x20 } }
-- The drain (`byte_80CF31C`, by side): the other side's whole area, the
-- silent element 5, 1 damage that can't counter (0x81), hitting by what it
-- is alone (row 0x52, target row 0), no spark, hit modifier 0.
local DRAIN_IS = define.collision { id = "anubis/drain", side0 = 0x80001080, side1 = 0x40001080, row_offset = 0x290 }
local DRAIN_ELEMENT, DRAIN_DAMAGE, DRAIN_COUNTER = 5, 1, 0x81

-- An Anubis statue's variant (`byte_80CF0C0`, by the chip's first
-- parameter).
function statue.variant(v: AnubisVariant): AnubisVariant
    return define.record("anubis-variant", v)
end

statue.anubis = statue.variant { anim = 0, period = 6, hp = 100, lifetime = 3000, name = 0xDB }
statue.poisphar = statue.variant { anim = 1, period = 1, hp = 100, lifetime = 1441, name = 0xDC }

-- `sub_80CF250`: drop; landed, take the panel if it is free solid ground
-- (the camera shaking), else break.
local function fall(me: Object)
    local p = me.pos
    local z = int.i32(p.z - me.vel.z)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    me.pos = Vec3.new(p.x, p.y, 0)
    if field.solid(me.panel_x, me.panel_y) then
        battle.shake_camera(2, 0x1E)
        if field.check(me.panel_x, me.panel_y, SOLID, OCCUPIED) then
            me.collision:set_region(regions.single)
            battle.play_sound(SOUND_LAND)
            me:set_action(8)
            return
        end
    end
    me.collision:set_region(nil)
    me.hp = 0
    me:set_action(2)
end

-- `sub_80CF332`: one of the other side's panels but its own (from the
-- bottom right), picked by a draw; nil when there is none.
local function bubble_panel(me: Object): (number?, number?)
    local rule = OTHER_SIDE[me.alliance]
    if not rule then
        error(string.format("the Anubis statue's side %d reads past byte_80CF364", me.alliance))
    end
    local panels = {}
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            if field.check(x, y, rule[1], rule[2]) and not (x == me.panel_x and y == me.panel_y) then
                table.insert(panels, bit32.bor(x, bit32.lshift(y, 4)))
            end
        end
    end
    if #panels == 0 then
        return nil, nil
    end
    local b = panels[battle.rng_positive() % #panels + 1]
    return bit32.band(b, 7), bit32.rshift(b, 4)
end

-- `sub_80CF2B4`: until time is up, a bubble every 16 ticks and the drain
-- every period.
local function poison(me: Object)
    if battle.time_up() then
        me:set_action(2)
        return
    end
    local s = me.state :: State
    local b = s.bubble_in
    s.bubble_in = int.u32(b - 1)
    if b <= 1 then
        s.bubble_in = BUBBLE_EVERY
        local x, y = bubble_panel(me)
        if x and y then
            local cx, cy = field.center(x, y)
            rising_bubble.spawn(Vec3.new(cx, cy, 0), 1)
        end
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t > 0 then
        return
    end
    me.timer2 = s.variant.period
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = DRAIN_ELEMENT,
        z = me.pos.z,
        region = regions.area[1 - me.alliance],
        hit_effect = 0xFF,
        target = collision.none,
        self_type = DRAIN_IS,
        damage = DRAIN_DAMAGE,
        stamina = DRAIN_COUNTER,
        hit_mod = 0,
    })
end

-- `sub_80CF1DC`: leave the field: absorbed, removed by a chip (a puff, or
-- blinking out), or broken (an explosion).
local function destroyed(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED_KIND)
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
    else
        battle.play_sound(SOUND_BREAK)
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.explosion)
    end
    me.visible = false
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- The statue's action table (`off_80CF1B8`): its own actions and the
-- framework's shared ones (a push knocks it back as far as the hit says).
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = fall,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = poison,
}

-- `sub_80CF18C`: hits (a push keeps the damage), the lifetime, the
-- reaction, its action; then present, and the sprite.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("Anubis statue action %d reads past its table", a))
        end
        action(me)
    end
    me.collision:present()
    me:update_sprite()
end

-- `sub_80CF0F0`: high over its panel, its variant's look, HP and
-- lifetime; its collision on no panel yet. On at once.
local function init(me: Object)
    local v = (me.state :: State).variant
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = v.anim, v.anim
    sprite:set_animation(v.anim)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, HEIGHT)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.name_id = v.name
    me.vel = Vec3.new(me.vel.x, me.vel.y, FALL_SPEED)
    me.element = 0
    me.timer = v.lifetime
    me.hp, me.max_hp = v.hp, v.hp
    ;(me.state :: State).bubble_in = 1
    me.timer2 = 1
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

statue.kind = define.kind {
    id = "anubis/statue",
    pool = "attack",
    state = { variant = "record:anubis-variant", bubble_in = "u32" },
    -- `sub_80CF0D0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80CF374`: a statue of `variant` on (x, y) with the damage word, of
-- `owner`'s side and facing, running while dimmed; its side's one field
-- object of class 1. (Its position is the spawner's registers until its
-- init places it.)
function statue.spawn(owner: Object, variant: AnubisVariant, x: number, y: number, damage: number): Object?
    local o = battle.spawn(statue.kind)
    if not o then
        return nil
    end
    (o.state :: State).variant = variant
    o.panel_x, o.panel_y = x, y
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    -- The registry's side is the whole alliance and flip halfword: a
    -- flipped owner's lands past the registry.
    if o.flip ~= 0 then
        error("sub_80CF374 registers a flipped user's Anubis statue past the field-object registry (alliance + 0x100)")
    end
    obstacle.register(o, o.alliance, 1)
    return o
end

return statue
