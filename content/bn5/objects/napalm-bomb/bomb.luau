--!strict
-- A napalm bomb (BN5's attack object #0x3A, 0x080CC490; BN6's `sub_80CB284` is the same, though no BN6 content
-- throws it): lobbed at a panel, fire, it lands there in 45 ticks, falling by its variant's gravity (variant 2's
-- lighter), its target marked in the battle flag 0x40 mode (objects/dice's marker). Off a solid panel it goes in a
-- puff. On one it hits its panel for 5 ticks (thrown, hitting navis, hit modifier 3, no spark), with a sound; when
-- nothing stands there, its fire spreads over the eight panels around too (a one-tick hit region) and burns on
-- them in an order the console's own RNG (RNG1) shuffles, half at once and half as it goes. Variant 0, unless told
-- quiet, lights the 3x3 around its target as it flies.

local collision = require("../../rules/collision")
local effects = require("@bn6/lib/effects")
local marker = require("../dice/marker")
local panels = require("@common/panels")
local regions = require("@common/regions")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- Its variant (its first parameter: 0, 1 or 2) and its second (nonzero: it doesn't light its target).
    variant: number,
    quiet: number,
    -- The panel it lands on (its third and fourth parameters).
    target_x: number,
    target_y: number,
    -- The panels its fire burns on, shuffled (+0x64, packed x | y << 4), and how many (+0x60).
    burning: { number },
    count: number,
}

local bomb = {}

local SPRITE = asset.sprite("bn5:napalm-bomb")
local SOUND_LOB = asset.sound("bn5:cannon")
local SOUND_LAND = asset.sound("bn5:target-move")
-- The fire on a panel (the effect table's row 0x4B: sprite 10-19).
local FIRE = define.effect { sprite = asset.sprite("bn5:napalm-fire"), anim = 0, palette = 0 }
-- Its flight's ticks, and its gravity by the variant (16.16 a tick).
local FLIGHT = 0x2D
local GRAVITY, LIGHT_GRAVITY = -0xA000, -0x4000
-- A panel something stands on (as one word).
local TAKEN = 0x0F80_0000
-- How long it hits.
local BURST = 4

local function gravity(me: Object): number
    return if (me.state :: State).variant == 2 then LIGHT_GRAVITY else GRAVITY
end

-- `sub_8000CA6` with RNG1: shuffle the first `n` panels with `n` swaps of two picked at random. Every console does
-- it with its own RNG; the simulation follows the local side's, and the other's draws alike.
local function shuffle(list: { number }, n: number)
    local here = battle.local_side()
    for _ = 1, n do
        local i = battle.console_rng_positive(here) % n + 1
        local j = battle.console_rng_positive(here) % n + 1
        list[i], list[j] = list[j], list[i]
        battle.console_rng_positive(bit32.bxor(here, 1))
        battle.console_rng_positive(bit32.bxor(here, 1))
    end
end

-- 0x080CC72E: the panels around its own (region 0x10, turned to its side) that exist, shuffled.
local function spread(me: Object)
    local s = me.state :: State
    local list = panels.region(me.panel_x, me.panel_y, regions.ring, panels.side_direction(me.alliance), 0, 0)
    s.count = #list
    if #list == 0 then
        return
    end
    shuffle(list, #list)
    for i, p in list do
        s.burning[i] = p
    end
end

-- 0x080CC75C: fire on the panels `first` to `last` of its list, on the ground at their centers.
local function burn(me: Object, first: number, last: number)
    local s = me.state :: State
    for i = first, last do
        local p = s.burning[i]
        local x, y = field.center(bit32.band(p, 7), bit32.rshift(p, 4))
        battle.effect(Vec3.new(x, y, 0), FIRE)
    end
end

-- Action 0 (0x080CC584): its flight, aimed on the first tick to land on its target in 45 ticks; the target marked
-- 30 and 15 ticks out; then it lands.
local function fly(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = FLIGHT
        local x, y = field.center(s.target_x, s.target_y)
        me.vel = trajectory.in_ticks(me.pos, x, y, 0, FLIGHT, gravity(me))
        me.phase_init = 4
    end
    if me.timer == 0x1E then
        marker.spawn(me, s.target_x, s.target_y, 0x1E, 0xA, true)
    elseif me.timer == 0xF then
        marker.spawn(me, s.target_x, s.target_y, 0xF, 0xC, true)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + gravity(me)))
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        if s.variant == 0 and s.quiet == 0 and bit32.band(me.timer, 4) ~= 0 then
            panels.highlight_region(s.target_x, s.target_y, regions.square, me.facing)
        end
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if not field.solid(me.panel_x, me.panel_y) then
        battle.effect(me.pos, effects.puff)
        me:set_lifecycle("destroy")
        return
    end
    me.collision:set_region(regions.single)
    if not field.solid(me.panel_x, me.panel_y) then
        battle.effect(me.pos, effects.puff)
    else
        if bit32.band(field.flags(me.panel_x, me.panel_y), TAKEN) == 0 then
            spread(me)
            local box = battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = me.element,
                z = me.pos.z,
                region = regions.ring,
                hit_effect = nil,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 3,
            })
            if box then
                box.run_while_dimmed = me.run_while_dimmed
            end
        end
        battle.play_sound(SOUND_LAND)
    end
    me.timer = BURST
    me:set_action(4)
end

-- Action 4 (0x080CC6B8): it bursts: on its first tick its region cleared, fire on its panel and on the first half
-- of the panels around; 5 ticks on, the rest, and it goes.
local function burst(me: Object)
    local s = me.state :: State
    local half = bit32.rshift(s.count, 1)
    if me.timer == BURST then
        me.collision:set_region(nil)
        battle.effect(me.pos, FIRE)
        if half ~= 0 then
            burn(me, 1, half)
        end
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if s.count - half ~= 0 then
        burn(me, half + 1, s.count)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = fly, [4] = burst }

-- 0x080CC52C: the hits against it resolve and spark; at the battle's end (unless dimmed) it goes in a puff; its
-- region cleared on a hit; its action.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if not battle.dimmed() and battle.over() then
        battle.effect(me.pos, effects.puff)
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("A napalm bomb's action %#x reads past its table (0x080CC57C)", me.action))
    end
    action(me)
    c:present()
end

-- 0x080CC4B4: its sprite (animation 1, its shadow), shown, facing its way, on the panel under it; its collision
-- (thrown, hitting navis, hit modifier 3, no spark), no region yet; its target marked; a sound; its first update.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 1, 1
    sprite:set_animation(1)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_panel_from_coordinates()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    local c = me.collision
    c:set_hit_effect(nil)
    c:set_region(nil)
    c:present()
    marker.spawn(me, s.target_x, s.target_y, FLIGHT, 1, false)
    battle.play_sound(SOUND_LOB)
    me:set_lifecycle("update")
    tick(me)
end

bomb.kind = define.kind {
    id = "bn5:napalm-bomb",
    pool = "attack",
    state = {
        variant = "u8",
        quiet = "u8",
        target_x = "u8",
        target_y = "u8",
        burning = "u8[8]",
        count = "u8",
    },
    -- 0x080CC490.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

export type Lob = {
    -- Its variant, and whether it keeps its target unlit.
    variant: number,
    quiet: boolean?,
    -- The panel it lands on.
    target_x: number,
    target_y: number,
    -- The damage word.
    damage: number,
    -- It runs while the battle is dimmed (0x080CC71A).
    while_dimmed: boolean?,
}

-- 0x080CC6FE: a bomb of `owner`'s side and facing at `pos`, fire, with the damage word, lobbed at its target;
-- 0x080CC71A: one that runs while dimmed (whose failed spawn returns through another routine's frame).
function bomb.lob(owner: Object, pos: Vec3, lob: Lob): Object?
    local o = battle.spawn(bomb.kind, pos)
    if not o then
        if lob.while_dimmed then
            error("a napalm bomb's failed spawn returns through 0x080CC758's frame (0x080CC71A)")
        end
        return nil
    end
    local s = o.state :: State
    s.variant = lob.variant
    s.quiet = if lob.quiet then 1 else 0
    s.target_x, s.target_y = lob.target_x, lob.target_y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.element = 1
    o:set_damage_word(lob.damage)
    if lob.while_dimmed then
        o.run_while_dimmed = true
    end
    return o
end

return bomb
