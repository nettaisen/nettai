--!strict
-- SunMoon's moonlight (the original's attack object #0xB6, `sub_80DC0E8`):
-- a shaft of light (SunMoon's sprite, animation 4) 40 pixels ahead of its
-- panel. After 3 ticks it hits the panel three columns ahead once with an
-- uninstall (damage 0x2000: a flag bit alone), then every tick for 100
-- ticks with its damage, a sound every 8 ticks; the hits resolve while
-- dimmed. See docs/engine/chips.md §3.6.32.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

export type State = {
    -- Ticks since its last sound, 0..7 (ExtraVars[0]).
    ticks: number,
}

local moon_beam = {}

local SPRITE = asset.sprite("moon-beam")
local ANIM = 4
local SOUND = asset.sound("moon-beam")

-- How far ahead of its panel it stands (whole pixels).
local AHEAD = 0x28

-- A hit on the panel three columns ahead that resolves while dimmed
-- (`sub_80C53A6`).
local function hit(me: Object, is: CollisionType, damage: number, hit_mod: number)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + me.facing * 3,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        -- None.
        hit_effect = 0xFF,
        target = collision.hits_navis,
        self_type = is,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = hit_mod,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80DC180`: 3 ticks.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 3
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(4)
    end
end

-- `sub_80DC1A2`: the uninstall, then 100 ticks of hits.
local function shine(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = 0x64
        me.timer2 = 0
        hit(me, collision.thrown_piercing_break, 0x2000, 0)
        me.phase_init = 4
    end
    if s.ticks == 0 then
        battle.play_sound(SOUND)
    end
    local t = s.ticks + 1
    s.ticks = if t >= 8 then 0 else t
    hit(me, collision.drain_break, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), 3)
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DC160`.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        wait(me)
    elseif a == 4 then
        shine(me)
    else
        error(string.format("SunMoon's moonlight action %#x reads past its table (off_80DC178)", a))
    end
    me:update_sprite()
end

-- `sub_80DC108`: its look, ahead of its panel on the ground; then its
-- first tick.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    sprite:step()
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD * 0x10000, p.y, 0)
    local s = me.state :: State
    s.ticks = 0
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80DC0E8`.
local function update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

moon_beam.kind = define.kind {
    id = "sunmoon/moon-beam",
    pool = "attack",
    state = { ticks = "u8" },
    update = update,
}

-- `sub_80DC22C`: moonlight on (x, y) with `element` and the damage word,
-- on `owner`'s side. It runs while dimmed. (Its position is the spawner's
-- registers, which its init overwrites.)
function moon_beam.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(moon_beam.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return moon_beam
